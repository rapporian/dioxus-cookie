//! Cross-platform cookie management for Dioxus fullstack applications.
//!
//! Dioxus apps can target web, desktop, iOS, and Android from a single codebase.
//! Native platforms lack built-in cookie storage—when a server function sets a cookie,
//! native apps silently discard it. **dioxus-cookie** provides a unified cookie API that
//! works identically across all platforms.
//!
//! # Quick Start
//!
//! ```rust,ignore
//! fn main() {
//!     dioxus_cookie::init();  // Call before dioxus::launch()
//!     dioxus::launch(App);
//! }
//!
//! #[server]
//! async fn login(credentials: Credentials) -> Result<(), ServerFnError> {
//!     dioxus_cookie::set("session", &token, &CookieOptions::default())?;
//!     Ok(())
//! }
//! ```
//!
//! # Platform Behavior
//!
//! | Platform | Storage |
//! |----------|---------|
//! | Server | HTTP `Set-Cookie` headers |
//! | Browser | `document.cookie` |
//! | Desktop | System keyring |
//! | iOS | Keychain |
//! | Android | Keystore |
//!
//! # Features
//!
//! - `server` — Server-side cookie handling via HTTP headers
//! - `desktop` — Desktop platforms with system keyring storage
//! - `mobile` — iOS/Android with Keychain/Keystore storage
//! - `mobile-sim` — Mobile + file fallback for iOS Simulator development
//! - `file-store` — Encrypted file fallback (see security note below)
//!
//! # File Storage Fallback
//!
//! The `file-store` feature provides encrypted file-based storage for environments
//! where the system keychain is unavailable (iOS Simulator, Linux without D-Bus,
//! CI/CD pipelines, Docker containers).
//!
//! **Security limitations:**
//! - **Debug builds only** — automatically disabled in release builds
//! - **Obfuscation, not protection** — deters casual inspection but does not
//!   protect against local attackers with file system access
//! - **Not for production** — production apps must use real keychain storage

#![deny(warnings)]
#![forbid(unsafe_code)]

mod types;

#[cfg(feature = "server")]
mod server;

#[cfg(all(feature = "keyring", not(feature = "server")))]
mod native;

#[cfg(all(feature = "keyring", feature = "file-store", not(feature = "server")))]
mod file_store;

#[cfg(all(not(feature = "server"), not(feature = "keyring")))]
mod stubs;

pub use types::*;

macro_rules! platform_dispatch {
    ($fn_name:ident($($arg:expr),*)) => {{
        #[cfg(feature = "server")]
        { server::$fn_name($($arg),*) }

        #[cfg(all(feature = "keyring", not(feature = "server")))]
        { native::$fn_name($($arg),*) }

        #[cfg(all(not(feature = "server"), not(feature = "keyring")))]
        { stubs::$fn_name($($arg),*) }
    }};
}

/// Initialize the cookie system.
///
/// Call this **before** `dioxus::launch()` in your `main()` function.
/// This sets up the platform-appropriate cookie storage backend.
///
/// - **Server**: No-op (cookies handled via HTTP headers)
/// - **Browser**: No-op (cookies handled via `document.cookie`)
/// - **Desktop/Mobile**: Initializes system keyring and configures HTTP client
///
/// # Example
///
/// ```rust,ignore
/// fn main() {
///     dioxus_cookie::init();
///     dioxus::launch(App);
/// }
/// ```
///
/// # Panics
///
/// Panics if called after `dioxus::launch()` has already initialized the HTTP client.
pub fn init() {
    #[cfg(all(feature = "keyring", not(feature = "server")))]
    native::init();
}

/// Retrieves a cookie value by name.
///
/// Returns `None` if:
/// - The cookie doesn't exist
/// - The cookie is `HttpOnly` (blocked for security)
/// - The cookie has expired
///
/// # Example
///
/// ```rust,ignore
/// if let Some(theme) = dioxus_cookie::get("theme") {
///     println!("User prefers: {}", theme);
/// }
/// ```
///
/// # HttpOnly Cookies
///
/// This function respects `HttpOnly` just like browsers do—it returns `None`
/// for HttpOnly cookies to prevent client-side access. Use [`get_internal`]
/// only for server-side session restoration.
pub fn get(name: &str) -> Option<String> {
    platform_dispatch!(get(name))
}

/// Retrieves a cookie value, bypassing the `HttpOnly` restriction.
///
/// **Warning**: Only use this for server-side operations like session restoration
/// on native platforms. Never expose HttpOnly cookie values to user-facing code.
///
/// # Example
///
/// ```rust,ignore
/// // In app initialization, restore session from stored cookie
/// if let Some(token) = dioxus_cookie::get_internal("session") {
///     restore_session(&token).await;
/// }
/// ```
pub fn get_internal(name: &str) -> Option<String> {
    platform_dispatch!(get_internal(name))
}

/// Sets a cookie with the given name, value, and options.
///
/// # Arguments
///
/// * `name` — Cookie name (should be alphanumeric with hyphens/underscores)
/// * `value` — Cookie value (will be URL-encoded automatically)
/// * `options` — Cookie attributes (expiration, security flags, etc.)
///
/// # Example
///
/// ```rust,ignore
/// use dioxus_cookie::{CookieOptions, SameSite};
/// use std::time::Duration;
///
/// #[server]
/// async fn login(credentials: Credentials) -> Result<User, ServerFnError> {
///     let user = authenticate(credentials).await?;
///
///     dioxus_cookie::set("session", &user.token, &CookieOptions {
///         max_age: Some(Duration::from_secs(86400 * 7)),
///         http_only: true,
///         secure: true,
///         same_site: SameSite::Strict,
///         path: "/".to_string(),
///     })?;
///
///     Ok(user)
/// }
/// ```
///
/// # Platform Behavior
///
/// - **Server**: Sets `Set-Cookie` HTTP header in the response
/// - **Browser**: Writes to `document.cookie`
/// - **Desktop/Mobile**: Stores in system keyring
pub fn set(name: &str, value: &str, options: &CookieOptions) -> Result<(), CookieError> {
    platform_dispatch!(set(name, value, options))
}

/// Deletes a cookie by name.
///
/// # Example
///
/// ```rust,ignore
/// #[server]
/// async fn logout() -> Result<(), ServerFnError> {
///     dioxus_cookie::clear("session")?;
///     Ok(())
/// }
/// ```
///
/// # Platform Behavior
///
/// - **Server**: Sets cookie with immediate expiration
/// - **Browser**: Removes from `document.cookie`
/// - **Desktop/Mobile**: Removes from system keyring
pub fn clear(name: &str) -> Result<(), CookieError> {
    platform_dispatch!(clear(name))
}

/// Lists names of all accessible cookies.
///
/// Returns only non-HttpOnly cookies (matching browser behavior).
/// Expired cookies are automatically excluded.
///
/// # Example
///
/// ```rust,ignore
/// let names = dioxus_cookie::list_names();
/// for name in names {
///     println!("Cookie: {}", name);
/// }
/// ```
pub fn list_names() -> Vec<String> {
    platform_dispatch!(list_names())
}

/// Returns the active storage backend type.
///
/// Useful for debugging and diagnostics.
///
/// # Returns
///
/// - `"server"` — HTTP headers (server-side)
/// - `"keychain"` — System keyring (desktop/mobile)
/// - `"file"` — Encrypted file fallback (iOS Simulator, debug only)
/// - `"browser"` — `document.cookie` (WASM)
/// - `"stub"` — No-op implementation
/// - `"uninitialized"` — [`init`] not yet called
pub fn get_storage_type() -> &'static str {
    #[cfg(feature = "server")]
    {
        "server"
    }

    #[cfg(all(feature = "keyring", not(feature = "server")))]
    {
        native::get_storage_type()
    }

    #[cfg(all(not(feature = "server"), not(feature = "keyring")))]
    {
        stubs::get_storage_type()
    }
}
