# dioxus-cookie

Unified cookie storage for [Dioxus](https://dioxuslabs.com/) fullstack apps that fills the gap in native platforms. One interface for web, desktop, iOS, and Android—no platform-specific code, no `#[cfg]` blocks, no silent auth failures. Includes encrypted file-vault fallback for consistent simulator development.

## The Problem

Dioxus fullstack apps compile to multiple targets from one codebase, but cookie handling is fragmented:

| Platform      | Cookie Mechanism          | What Happens           |
| ------------- | ------------------------- | ---------------------- |
| Web (server)  | HTTP `Set-Cookie` headers | Works                  |
| Web (WASM)    | `document.cookie`         | Works                  |
| Desktop       | None                      | Cookies silently lost  |
| iOS/Android   | None                      | Cookies silently lost  |
| iOS Simulator | Keychain blocked          | Entitlement errors     |

When your server function sets a session cookie, it works perfectly on web. But on desktop and mobile, that cookie vanishes into the void—your auth breaks with no error message.

## The Solution

**dioxus-cookie** bridges this gap with platform-appropriate storage:

- **Server**: HTTP headers (standard web cookies)
- **Browser**: `document.cookie` (standard web cookies)
- **Desktop**: System keyring (macOS Keychain, Windows Credential Manager, Linux Secret Service)
- **iOS**: Keychain
- **Android**: Secure storage via keyring
- **iOS Simulator**: Encrypted file fallback (debug builds only)

One API. All platforms. Your auth code stays clean.

## Quick Start

**1. Add to `Cargo.toml`:**

```toml
[dependencies]
dioxus-cookie = { version = "0.1", features = ["server", "desktop"] }
```

**2. Initialize before launch:**

```rust
use dioxus_cookie as cookie;

fn main() {
    cookie::init();
    dioxus::launch(App);
}
```

**3. Use in server functions:**

```rust
use dioxus_cookie::{self as cookie, CookieOptions, SameSite};
use std::time::Duration;

#[server]
async fn login(credentials: Credentials) -> Result<User, ServerFnError> {
    let user = authenticate(credentials).await?;

    cookie::set("session", &user.token, &CookieOptions::default())?;
    Ok(user)
}

#[server]
async fn logout() -> Result<(), ServerFnError> {
    cookie::clear("session")?;
    Ok(())
}
```

That's it. Cookies now work on web, desktop, iOS, and Android.

## Installation

### Feature Flags

Enable features for all platforms your app targets. The correct backend is selected automatically at compile time based on the build target.

**Typical fullstack app (web + desktop):**

```toml
[dependencies]
dioxus-cookie = { version = "0.1", features = ["server", "desktop"] }
```

**Fullstack app with mobile:**

```toml
[dependencies]
dioxus-cookie = { version = "0.1", features = ["server", "desktop", "mobile"] }
```

**iOS Simulator development** (add `mobile-sim` for file-based fallback):

```toml
[dependencies]
dioxus-cookie = { version = "0.1", features = ["server", "mobile-sim"] }
```

### How Features Map to Build Targets

When you run `dx build` or `dx serve`, the correct backend is selected automatically based on the build target:

| Build command                                   | Backend used                          |
| ----------------------------------------------- | ------------------------------------- |
| `dx serve` / `dx build --platform web` (server) | HTTP headers (`server` feature)       |
| `dx build --platform web` (WASM client)         | `document.cookie` (no feature needed) |
| `dx build --platform desktop`                   | System keyring (`desktop` feature)    |
| `dx build --platform ios`                       | iOS Keychain (`mobile` feature)       |
| `dx build --platform android`                   | Android Keystore (`mobile` feature)   |

You enable all the features your project needs in `Cargo.toml`, then each `dx build` target uses the appropriate backend.

### All Features

| Feature      | Description                                                                                            |
| ------------ | ------------------------------------------------------------------------------------------------------ |
| `server`     | Server-side cookie handling via HTTP headers                                                           |
| `desktop`    | Desktop platforms with system keyring storage                                                          |
| `mobile`     | iOS/Android with Keychain/Keystore storage                                                             |
| `mobile-sim` | Mobile + file fallback for iOS Simulator development                                                   |
| `file-store` | Encrypted file fallback (debug builds only, see [Security](#file-storage-fallback-file-store-feature)) |

## Usage

### Setting Cookies

```rust
use dioxus_cookie::{self as cookie, CookieOptions, SameSite};
use std::time::Duration;

#[server]
async fn login(credentials: Credentials) -> Result<User, ServerFnError> {
    let user = authenticate(credentials).await?;

    // Simple: use secure defaults
    cookie::set("session", &user.token, &CookieOptions::default())?;

    // Or customize options
    cookie::set("session", &user.token, &CookieOptions {
        max_age: Some(Duration::from_secs(86400 * 7)),  // 7 days
        http_only: true,
        secure: true,
        same_site: SameSite::Strict,
        path: "/".to_string(),
    })?;

    Ok(user)
}
```

### Reading Cookies

```rust
use dioxus_cookie as cookie;

// Returns None for HttpOnly cookies (by design)
let preference = cookie::get("theme");

// List all accessible cookie names
let names = cookie::list_names();
```

### Clearing Cookies

```rust
use dioxus_cookie as cookie;

#[server]
async fn logout() -> Result<(), ServerFnError> {
    cookie::clear("session")?;
    Ok(())
}
```

## Security

### Secure Defaults

`CookieOptions::default()` prioritizes security:

| Option      | Default | Purpose                    |
| ----------- | ------- | -------------------------- |
| `http_only` | `true`  | Prevents JavaScript access |
| `secure`    | `true`  | HTTPS only                 |
| `same_site` | `Lax`   | CSRF protection            |
| `path`      | `"/"`   | Available to all routes    |

### HttpOnly Enforcement

On native platforms, dioxus-cookie enforces HttpOnly the same way browsers do:

```rust
use dioxus_cookie as cookie;

cookie::get("session")           // → None (HttpOnly blocked)
cookie::get_internal("session")  // → Some(...) (internal use only)
```

HttpOnly cookies are still sent automatically with HTTP requests.

### File Storage Fallback (`file-store` feature)

The `file-store` feature provides an encrypted file-based fallback for environments where the system keychain is unavailable:

| Environment         | Why keychain fails            |
| ------------------- | ----------------------------- |
| iOS Simulator       | Missing keychain entitlements |
| Linux without D-Bus | No Secret Service available   |
| CI/CD pipelines     | No desktop session            |
| Docker containers   | No keychain access            |

**Security limitations:**

- **DEBUG BUILDS ONLY** — Automatically disabled in release builds
- **Obfuscation, not protection** — AES-256-GCM encryption with a machine-derived key deters casual inspection but does not protect against local attackers with file access
- **Not for production** — Production apps must use real keychain storage

Use `mobile-sim` (which includes `file-store`) for iOS Simulator development, or add `file-store` directly for other fallback scenarios.

## Architecture

```
┌─────────────────────────────────────────────────────┐
│                dioxus-cookie API                    │
│            get / set / clear / list_names           │
└────────────────────────┬────────────────────────────┘
                         │
      ┌──────────────────┼──────────────────┐
      ▼                  ▼                  ▼
┌───────────┐      ┌───────────┐      ┌───────────┐
│  server   │      │  native   │      │   stubs   │
│  module   │      │  module   │      │  module   │
└─────┬─────┘      └─────┬─────┘      └─────┬─────┘
      │                  │                  │
      ▼                  ▼                  ▼
┌───────────┐      ┌───────────┐      ┌───────────┐
│ HTTP      │      │ System    │      │ document  │
│ Headers   │      │ Keyring   │      │ .cookie   │
└───────────┘      └───────────┘      └───────────┘
```

## API Reference

| Function                    | Description                                                   |
| --------------------------- | ------------------------------------------------------------- |
| `init()`                    | Initialize the cookie system. Call before `dioxus::launch()`. |
| `get(name)`                 | Get a cookie value. Returns `None` for HttpOnly cookies.      |
| `get_internal(name)`        | Get a cookie bypassing HttpOnly (internal use).               |
| `set(name, value, options)` | Set a cookie with the given options.                          |
| `clear(name)`               | Delete a cookie.                                              |
| `list_names()`              | List all accessible cookie names.                             |
| `get_storage_type()`        | Get the active storage backend.                               |

## Troubleshooting

**"No server context available"**
Cookie operations that set HTTP headers must run inside `#[server]` functions.

**Cookies not persisting on iOS Simulator**
Use the `mobile-sim` feature. The simulator lacks keychain entitlements, so cookies fall back to encrypted file storage.

**"GLOBAL_REQUEST_CLIENT already initialized"**
Call `cookie::init()` before `dioxus::launch()`.

**`get()` returns `None` for a cookie that was set**
The cookie is HttpOnly. This is intentional—use `get_internal()` only for session restoration.

## License

MIT
