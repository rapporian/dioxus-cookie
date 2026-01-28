use std::collections::HashMap;
use std::sync::{Arc, OnceLock, RwLock};

use cookie_store::CookieStore;
use dioxus::fullstack::http::HeaderValue;
use dioxus::fullstack::reqwest::{self, cookie::CookieStore as ReqwestCookieStore, Url};
use serde::{Deserialize, Serialize};

use crate::types::{CookieError, CookieOptions};

const SERVICE: &str = "com.dioxus-cookie.vault";
const KEY: &str = "cookie_vault_v3";
const DEFAULT_URL: &str = "https://app.local/";

#[cfg(feature = "file-store")]
const VAULT_FILE: &str = "cookie_vault_v3.json";

#[derive(Serialize, Deserialize, Default)]
struct CookieVault {
    cookies: CookieStore,
    http_only: HashMap<String, bool>,
}

pub struct KeyringCookieStore {
    vault: RwLock<CookieVault>,
    #[cfg(feature = "file-store")]
    using_file_fallback: RwLock<bool>,
}

static STORE: OnceLock<Arc<KeyringCookieStore>> = OnceLock::new();

impl KeyringCookieStore {
    #[cfg(feature = "file-store")]
    fn new(force_file: bool) -> Arc<Self> {
        let (vault, using_file_fallback) = Self::load(force_file);
        Arc::new(Self {
            vault: RwLock::new(vault),
            using_file_fallback: RwLock::new(using_file_fallback),
        })
    }

    #[cfg(not(feature = "file-store"))]
    fn new() -> Arc<Self> {
        let vault = Self::load();
        Arc::new(Self {
            vault: RwLock::new(vault),
        })
    }

    #[cfg(not(feature = "file-store"))]
    fn load() -> CookieVault {
        keyring::Entry::new(SERVICE, KEY)
            .ok()
            .and_then(|e| e.get_password().ok())
            .and_then(|json| serde_json::from_str(&json).ok())
            .unwrap_or_default()
    }

    #[cfg(feature = "file-store")]
    fn load(force_file: bool) -> (CookieVault, bool) {
        // If forcing file mode, skip keyring entirely
        if force_file {
            if let Some(vault) = crate::file_store::read_vault(VAULT_FILE)
                .and_then(|json| serde_json::from_str(&json).ok())
            {
                return (vault, true);
            }
            return (CookieVault::default(), true);
        }

        // Try file first (might be from previous simulator run)
        if let Some(vault) = crate::file_store::read_vault(VAULT_FILE)
            .and_then(|json| serde_json::from_str(&json).ok())
        {
            return (vault, true);
        }

        // Try keyring
        match keyring::Entry::new(SERVICE, KEY) {
            Ok(entry) => match entry.get_password() {
                Ok(json) => {
                    if let Ok(vault) = serde_json::from_str(&json) {
                        return (vault, false);
                    }
                }
                Err(keyring::Error::NoEntry) => {}
                Err(e) => {
                    if Self::is_keyring_unavailable(&e.to_string()) {
                        return (CookieVault::default(), true);
                    }
                }
            },
            Err(e) => {
                if Self::is_keyring_unavailable(&e.to_string()) {
                    return (CookieVault::default(), true);
                }
            }
        }

        (CookieVault::default(), false)
    }

    fn persist(&self) {
        let vault = self.vault.read().unwrap();
        let Ok(json) = serde_json::to_string(&*vault) else {
            return;
        };

        #[cfg(feature = "file-store")]
        {
            if *self.using_file_fallback.read().unwrap() {
                crate::file_store::write_vault(&json, VAULT_FILE);
                return;
            }
        }

        if let Ok(entry) = keyring::Entry::new(SERVICE, KEY) {
            if entry.set_password(&json).is_err() {
                #[cfg(feature = "file-store")]
                {
                    *self.using_file_fallback.write().unwrap() = true;
                    crate::file_store::write_vault(&json, VAULT_FILE);
                }
            }
        }
    }

    #[cfg(feature = "file-store")]
    fn is_keyring_unavailable(err: &str) -> bool {
        let err_lower = err.to_lowercase();
        err_lower.contains("entitlement")
            || err_lower.contains("platform secure storage")
            || err_lower.contains("jni")
            || err_lower.contains("keystore")
            || err_lower.contains("ndk-context")
    }
}

impl KeyringCookieStore {
    fn get_cookie_value(&self, vault: &CookieVault, name: &str) -> Option<String> {
        vault
            .cookies
            .iter_unexpired()
            .find(|c| c.name() == name)
            .and_then(|c| urlencoding::decode(c.value()).ok())
            .map(|s| s.into_owned())
    }

    fn get(&self, name: &str) -> Option<String> {
        let vault = self.vault.read().unwrap();
        if vault.http_only.get(name) == Some(&true) {
            return None;
        }
        self.get_cookie_value(&vault, name)
    }

    fn get_internal(&self, name: &str) -> Option<String> {
        let vault = self.vault.read().unwrap();
        self.get_cookie_value(&vault, name)
    }

    fn set(&self, name: &str, value: &str, options: &CookieOptions) -> Result<(), CookieError> {
        let url = Url::parse(DEFAULT_URL).expect("Default URL should be valid");

        {
            let mut vault = self.vault.write().unwrap();
            let _ = vault
                .cookies
                .parse(&options.build_header(name, value), &url);

            if options.http_only {
                vault.http_only.insert(name.to_string(), true);
            } else {
                vault.http_only.remove(name);
            }
        }

        self.persist();
        Ok(())
    }

    fn clear(&self, name: &str) -> Result<(), CookieError> {
        {
            let mut vault = self.vault.write().unwrap();
            vault.http_only.remove(name);

            let to_remove: Option<(String, String)> = vault
                .cookies
                .iter_unexpired()
                .find(|c| c.name() == name)
                .map(|c| {
                    (
                        c.domain().unwrap_or("app.local").to_string(),
                        c.path().unwrap_or("/").to_string(),
                    )
                });

            if let Some((domain, path)) = to_remove {
                vault.cookies.remove(&domain, &path, name);
            }
        }

        self.persist();
        Ok(())
    }

    fn list_names(&self) -> Vec<String> {
        let vault = self.vault.read().unwrap();
        vault
            .cookies
            .iter_unexpired()
            .filter(|c| vault.http_only.get(c.name()) != Some(&true))
            .map(|c| c.name().to_string())
            .collect()
    }

    fn storage_type(&self) -> &'static str {
        #[cfg(feature = "file-store")]
        {
            if *self.using_file_fallback.read().unwrap() {
                #[cfg(target_os = "android")]
                return "android-file";

                #[cfg(not(target_os = "android"))]
                return "file";
            }
        }

        #[cfg(target_os = "android")]
        return "android-keystore";

        #[cfg(not(target_os = "android"))]
        "keychain"
    }
}

impl ReqwestCookieStore for KeyringCookieStore {
    fn set_cookies(&self, headers: &mut dyn Iterator<Item = &HeaderValue>, url: &Url) {
        {
            let mut vault = self.vault.write().unwrap();
            for header in headers {
                if let Ok(s) = header.to_str() {
                    let _ = vault.cookies.parse(s, url);

                    if let Some(name) = extract_cookie_name(s) {
                        if s.to_lowercase().contains("httponly") {
                            vault.http_only.insert(name, true);
                        } else {
                            vault.http_only.remove(&name);
                        }
                    }
                }
            }
        }
        self.persist();
    }

    fn cookies(&self, url: &Url) -> Option<HeaderValue> {
        let vault = self.vault.read().unwrap();
        let cookies: Vec<_> = vault
            .cookies
            .get_request_values(url)
            .map(|(n, v)| format!("{}={}", n, v))
            .collect();

        if cookies.is_empty() {
            None
        } else {
            HeaderValue::from_str(&cookies.join("; ")).ok()
        }
    }
}

pub fn init() {
    // Determine if we should force file mode on Android
    #[allow(unused_mut, unused_variables, unused_assignments)]
    let mut force_file = false;

    #[cfg(target_os = "android")]
    {
        // android-file feature: skip KeyStore entirely, use file storage
        #[cfg(feature = "android-file")]
        {
            force_file = true;
        }

        // Otherwise, try to initialize Android keyring
        #[cfg(not(feature = "android-file"))]
        {
            if let Err(e) = android_keyring::set_android_keyring_credential_builder() {
                #[cfg(feature = "file-store")]
                {
                    force_file = true;
                    eprintln!("dioxus-cookie: Android KeyStore init failed: {e}. Using file fallback.");
                }
                #[cfg(not(feature = "file-store"))]
                panic!("dioxus-cookie: Android KeyStore init failed: {e}. Enable file-store or android-file feature for fallback.");
            }
        }
    }

    // Initialize the cookie store
    #[cfg(feature = "file-store")]
    let store = STORE.get_or_init(|| KeyringCookieStore::new(force_file));

    #[cfg(not(feature = "file-store"))]
    let store = STORE.get_or_init(KeyringCookieStore::new);

    let client = reqwest::Client::builder()
        .cookie_store(true)
        .cookie_provider(store.clone())
        .danger_accept_invalid_certs(true) // For mkcert in dev
        .build()
        .expect("Failed to build reqwest client");

    let _ = dioxus::fullstack::GLOBAL_REQUEST_CLIENT.set(client);
}

/// Returns None for HttpOnly cookies.
pub fn get(name: &str) -> Option<String> {
    STORE.get()?.get(name)
}

/// Bypasses HttpOnly check.
pub fn get_internal(name: &str) -> Option<String> {
    STORE.get()?.get_internal(name)
}

pub fn set(name: &str, value: &str, options: &CookieOptions) -> Result<(), CookieError> {
    STORE
        .get()
        .ok_or_else(|| CookieError::new("Cookie store not initialized"))?
        .set(name, value, options)
}

pub fn clear(name: &str) -> Result<(), CookieError> {
    STORE
        .get()
        .ok_or_else(|| CookieError::new("Cookie store not initialized"))?
        .clear(name)
}

pub fn list_names() -> Vec<String> {
    STORE.get().map(|s| s.list_names()).unwrap_or_default()
}

pub fn get_storage_type() -> &'static str {
    STORE
        .get()
        .map(|s| s.storage_type())
        .unwrap_or("uninitialized")
}

fn extract_cookie_name(header: &str) -> Option<String> {
    header
        .split(';')
        .next()?
        .split_once('=')
        .map(|(n, _)| n.trim().to_string())
}
