use std::time::Duration;

/// Cross-site request cookie policy (SameSite attribute).
///
/// Controls when cookies are sent with cross-site requests, providing
/// protection against CSRF attacks.
///
/// # Variants
///
/// - `Strict` — Cookie only sent with same-site requests
/// - `Lax` — Cookie sent with same-site requests and top-level navigation (default)
/// - `None` — Cookie sent with all requests (requires `Secure` flag)
#[derive(Clone, Debug, Default)]
pub enum SameSite {
    /// Cookie only sent with same-site requests.
    ///
    /// Most restrictive. Use for sensitive operations like banking or account changes.
    Strict,
    /// Cookie sent with same-site requests and top-level navigation.
    ///
    /// Balanced protection. Good for most session cookies.
    #[default]
    Lax,
    /// Cookie sent with all requests, including cross-site.
    ///
    /// Requires `Secure` flag. Use only when cross-site access is necessary
    /// (e.g., embedded widgets, third-party integrations).
    None,
}

impl SameSite {
    /// Returns the string representation for use in cookie headers.
    pub fn as_str(&self) -> &'static str {
        match self {
            SameSite::Strict => "Strict",
            SameSite::Lax => "Lax",
            SameSite::None => "None",
        }
    }
}

/// Configuration options for setting cookies.
///
/// The default options prioritize security:
/// - `http_only: true` — prevents JavaScript access
/// - `secure: true` — HTTPS only
/// - `same_site: Lax` — CSRF protection
/// - `path: "/"` — available to all routes
///
/// # Example
///
/// ```rust,ignore
/// use dioxus_cookie::{CookieOptions, SameSite};
/// use std::time::Duration;
///
/// let options = CookieOptions {
///     max_age: Some(Duration::from_secs(86400 * 7)), // 7 days
///     http_only: true,
///     secure: true,
///     same_site: SameSite::Strict,
///     path: "/".to_string(),
/// };
/// ```
#[derive(Clone, Debug)]
pub struct CookieOptions {
    /// Cookie lifetime. `None` = session cookie (deleted when browser closes).
    pub max_age: Option<Duration>,
    /// If `true`, cookie is inaccessible to JavaScript/WASM. Default: `true`.
    ///
    /// Always use `true` for session tokens to prevent XSS attacks.
    pub http_only: bool,
    /// If `true`, cookie is only sent over HTTPS. Default: `true`.
    pub secure: bool,
    /// Cross-site request policy. Default: [`SameSite::Lax`].
    pub same_site: SameSite,
    /// URL path scope for the cookie. Default: `"/"`.
    pub path: String,
}

impl Default for CookieOptions {
    fn default() -> Self {
        Self {
            max_age: None,
            http_only: true,
            secure: true,
            same_site: SameSite::Lax,
            path: "/".to_string(),
        }
    }
}

impl CookieOptions {
    /// Builds a `Set-Cookie` header string from the options.
    ///
    /// The value is URL-encoded automatically.
    pub fn build_header(&self, name: &str, value: &str) -> String {
        let mut s = format!("{}={}", name, urlencoding::encode(value));

        if self.http_only {
            s.push_str("; HttpOnly");
        }
        if self.secure {
            s.push_str("; Secure");
        }
        s.push_str("; SameSite=");
        s.push_str(self.same_site.as_str());
        s.push_str("; Path=");
        s.push_str(&self.path);

        if let Some(max_age) = self.max_age {
            s.push_str("; Max-Age=");
            s.push_str(&max_age.as_secs().to_string());
        }

        s
    }
}

/// Error type for cookie operations.
///
/// On the server, this automatically converts to `ServerFnError`
/// for seamless error propagation from `#[server]` functions.
#[derive(Debug, Clone, thiserror::Error)]
#[error("CookieError: {message}")]
pub struct CookieError {
    /// Human-readable error description.
    pub message: String,
}

impl CookieError {
    /// Creates a new cookie error with the given message.
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

#[cfg(feature = "server")]
impl From<CookieError> for dioxus::prelude::ServerFnError {
    fn from(err: CookieError) -> Self {
        dioxus::prelude::ServerFnError::new(err.message)
    }
}
