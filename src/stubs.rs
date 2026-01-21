use crate::types::{CookieError, CookieOptions};

#[cfg(all(
    target_arch = "wasm32",
    not(feature = "server"),
    not(feature = "keyring")
))]
mod wasm {
    use super::*;
    use web_sys::wasm_bindgen::JsCast;

    fn get_html_document() -> Option<web_sys::HtmlDocument> {
        web_sys::window()
            .and_then(|w| w.document())
            .and_then(|d| d.dyn_into::<web_sys::HtmlDocument>().ok())
    }

    fn get_document_cookie() -> Option<String> {
        get_html_document().and_then(|d| d.cookie().ok())
    }

    fn set_document_cookie(cookie_str: &str) -> Result<(), CookieError> {
        get_html_document()
            .ok_or_else(|| CookieError::new("No document available"))?
            .set_cookie(cookie_str)
            .map_err(|_| CookieError::new("Failed to set cookie"))
    }

    pub fn get(name: &str) -> Option<String> {
        let cookie_str = get_document_cookie()?;
        let prefix = format!("{}=", name);

        for cookie in cookie_str.split(';') {
            let cookie = cookie.trim();
            if let Some(value) = cookie.strip_prefix(&prefix) {
                return urlencoding::decode(value).ok().map(|s| s.into_owned());
            }
        }

        None
    }

    /// Same as get() — browser can't access HttpOnly cookies anyway.
    pub fn get_internal(name: &str) -> Option<String> {
        get(name)
    }

    pub fn set(name: &str, value: &str, options: &CookieOptions) -> Result<(), CookieError> {
        let encoded_value = urlencoding::encode(value);
        let mut parts = vec![format!("{}={}", name, encoded_value)];

        if options.secure {
            parts.push("Secure".to_string());
        }
        parts.push(format!("SameSite={}", options.same_site.as_str()));
        parts.push(format!("Path={}", options.path));

        if let Some(max_age) = options.max_age {
            parts.push(format!("Max-Age={}", max_age.as_secs()));
        }

        set_document_cookie(&parts.join("; "))
    }

    pub fn clear(name: &str) -> Result<(), CookieError> {
        set_document_cookie(&format!("{}=; Path=/; Max-Age=0", name))
    }

    pub fn list_names() -> Vec<String> {
        let Some(cookie_str) = get_document_cookie() else {
            return Vec::new();
        };

        cookie_str
            .split(';')
            .filter_map(|cookie| {
                let cookie = cookie.trim();
                cookie.split('=').next().map(|name| name.to_string())
            })
            .collect()
    }

    pub fn get_storage_type() -> &'static str {
        "browser"
    }
}

#[cfg(all(
    not(target_arch = "wasm32"),
    not(feature = "server"),
    not(feature = "keyring")
))]
mod native {
    use super::*;

    pub fn get(_name: &str) -> Option<String> {
        None
    }

    pub fn get_internal(_name: &str) -> Option<String> {
        None
    }

    pub fn set(_name: &str, _value: &str, _options: &CookieOptions) -> Result<(), CookieError> {
        Ok(())
    }

    pub fn clear(_name: &str) -> Result<(), CookieError> {
        Ok(())
    }

    pub fn list_names() -> Vec<String> {
        vec![]
    }

    pub fn get_storage_type() -> &'static str {
        "stub"
    }
}

#[cfg(all(
    target_arch = "wasm32",
    not(feature = "server"),
    not(feature = "keyring")
))]
pub use wasm::*;

#[cfg(all(
    not(target_arch = "wasm32"),
    not(feature = "server"),
    not(feature = "keyring")
))]
pub use native::*;
