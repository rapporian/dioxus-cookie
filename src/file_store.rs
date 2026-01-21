//! Encrypted file-based cookie storage fallback.
//!
//! Use cases:
//! - iOS Simulator (no keychain entitlements)
//! - Linux without D-Bus/Secret Service
//! - CI/CD pipelines and containers
//!
//! Security: AES-256-GCM with machine-derived key. Provides obfuscation,
//! not protection against local attackers. DEBUG BUILDS ONLY.
//!
//! Production apps must use real keychain storage (`keyring` feature).

use std::fs;
use std::path::PathBuf;

use aes_gcm::{
    aead::{Aead, KeyInit},
    Aes256Gcm, Nonce,
};
use sha2::{Digest, Sha256};

fn derive_key() -> Option<[u8; 32]> {
    let data = dirs::data_local_dir()?;
    let mut hasher = Sha256::new();
    hasher.update(b"dioxus-cookie-vault-v2");
    hasher.update(data.to_string_lossy().as_bytes());
    Some(hasher.finalize().into())
}

fn storage_dir() -> Option<PathBuf> {
    #[cfg(target_os = "ios")]
    {
        if let Some(home) = dirs::home_dir() {
            return Some(home.join("Documents").join(".dioxus_cookie_vault"));
        }
    }

    #[cfg(target_os = "android")]
    {
        if let Some(data) = dirs::data_local_dir() {
            return Some(data.join(".dioxus_cookie_vault"));
        }
    }

    #[cfg(not(any(target_os = "ios", target_os = "android")))]
    {
        if let Some(data) = dirs::data_local_dir() {
            return Some(data.join("dioxus-cookie").join("cookies"));
        }
    }

    None
}

fn ensure_storage_dir() -> Option<PathBuf> {
    let dir = storage_dir()?;
    if !dir.exists() {
        fs::create_dir_all(&dir).ok()?;
    }
    Some(dir)
}

/// Debug builds only.
fn is_file_storage_allowed() -> bool {
    cfg!(debug_assertions)
}

#[allow(deprecated)] // generic_array 0.x deprecation warnings
fn encrypt_vault(plaintext: &str) -> Option<Vec<u8>> {
    let key = derive_key()?;
    let cipher = Aes256Gcm::new_from_slice(&key).ok()?;

    let nonce_bytes: [u8; 12] = rand::random();
    let nonce = Nonce::from_slice(&nonce_bytes);

    let ciphertext = cipher.encrypt(nonce, plaintext.as_bytes()).ok()?;

    let mut result = nonce_bytes.to_vec();
    result.extend(ciphertext);
    Some(result)
}

#[allow(deprecated)] // generic_array 0.x deprecation warnings
fn decrypt_vault(data: &[u8]) -> Option<String> {
    if data.len() < 28 {
        return None;
    }

    let key = derive_key()?;
    let cipher = Aes256Gcm::new_from_slice(&key).ok()?;

    let (nonce_bytes, ciphertext) = data.split_at(12);
    let nonce = Nonce::from_slice(nonce_bytes);

    let plaintext = cipher.decrypt(nonce, ciphertext).ok()?;
    String::from_utf8(plaintext).ok()
}

pub fn write_vault(json: &str, filename: &str) -> bool {
    if !is_file_storage_allowed() {
        return false;
    }

    let dir = match ensure_storage_dir() {
        Some(d) => d,
        None => return false,
    };

    let encrypted = match encrypt_vault(json) {
        Some(data) => data,
        None => return false,
    };

    fs::write(dir.join(filename), &encrypted).is_ok()
}

pub fn read_vault(filename: &str) -> Option<String> {
    if !is_file_storage_allowed() {
        return None;
    }

    let dir = storage_dir()?;
    let path = dir.join(filename);

    if !path.exists() {
        return None;
    }

    let data = fs::read(&path).ok()?;
    decrypt_vault(&data)
}
