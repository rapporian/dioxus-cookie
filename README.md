# dioxus-cookie

Unified cookie storage for [Dioxus](https://dioxuslabs.com/) fullstack apps. Cookies set on the server—including secure `HttpOnly` session tokens—work on web but silently vanish on desktop and mobile. **dioxus-cookie** fixes that. One interface for all platforms, no `#[cfg]` blocks. Includes encrypted file-vault fallback for simulator/emulator development.

## The Problem

Dioxus fullstack apps compile to multiple targets from one codebase, but cookie handling is fragmented:

| Platform         | Cookie Mechanism          | What Happens          |
| ---------------- | ------------------------- | --------------------- |
| Web (server)     | HTTP `Set-Cookie` headers | Works                 |
| Web (WASM)       | `document.cookie`         | Works                 |
| Desktop          | None                      | Cookies silently lost |
| iOS              | None                      | Cookies silently lost |
| Android          | None                      | Cookies silently lost |
| iOS Simulator    | Keychain blocked          | Entitlement errors    |
| Android Emulator | KeyStore may fail         | Device-dependent      |

When your server function sets a cookie, it works perfectly on web. But on desktop and mobile, that cookie vanishes—whether it's a session token, user preference, or any other state.

## The Solution

**dioxus-cookie** bridges this gap with platform-appropriate storage:

- **Server**: HTTP headers (standard web cookies)
- **Browser**: `document.cookie` (standard web cookies)
- **Desktop**: System keyring (macOS Keychain, Windows Credential Manager, Linux Secret Service)
- **iOS**: Keychain (via keyring crate)
- **Android**: KeyStore (experimental, via android-keyring crate)
- **Simulators/Emulators**: Encrypted file fallback (debug builds only)

One API. Your cookie code stays clean—auth, preferences, everything.

## Quick Start

**1. Add to `Cargo.toml`:**

```toml
[dependencies]
dioxus-cookie = { version = "0.2", features = ["server", "desktop"] }
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
use dioxus_cookie::{self as cookie, CookieOptions};

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
dioxus-cookie = { version = "0.2", features = ["server", "desktop"] }
```

**Fullstack app with iOS/Android:**

```toml
[dependencies]
dioxus-cookie = { version = "0.2", features = ["server", "desktop", "mobile"] }
```

**Simulator/Emulator development** (add `mobile-sim` for file-based fallback):

```toml
[dependencies]
dioxus-cookie = { version = "0.2", features = ["server", "mobile-sim"] }
```

### How Features Map to Build Targets

When you run `dx build` or `dx serve`, the correct backend is selected automatically based on the build target:

| Build command                                   | Backend used                                      |
| ----------------------------------------------- | ------------------------------------------------- |
| `dx serve` / `dx build --platform web` (server) | HTTP headers (`server` feature)                   |
| `dx build --platform web` (WASM client)         | `document.cookie` (no feature needed)             |
| `dx build --platform desktop`                   | System keyring (`desktop` feature)                |
| `dx build --platform ios`                       | iOS Keychain (`mobile` feature)                   |
| `dx build --platform android`                   | Android KeyStore (`mobile` feature, experimental) |

You enable all the features your project needs in `Cargo.toml`, then each `dx build` target uses the appropriate backend.

### All Features

| Feature      | Description                                                                                            |
| ------------ | ------------------------------------------------------------------------------------------------------ |
| `server`     | Server-side cookie handling via HTTP headers                                                           |
| `desktop`    | Desktop platforms with system keyring storage                                                          |
| `mobile`     | iOS/Android with Keychain/KeyStore storage (Android is experimental)                                   |
| `mobile-sim` | Mobile + file fallback for simulator/emulator development                                              |
| `file-store` | Encrypted file fallback (debug builds only, see [Security](#file-storage-fallback-file-store-feature)) |
| `android-file` | Force encrypted file storage on Android (skip KeyStore for maximum reliability) |

## Android Platform Details

Android support uses the experimental [`android-keyring`](https://crates.io/crates/android-keyring) crate to access Android's KeyStore. While KeyStore provides the most secure storage, there are important reliability considerations.

### Why Android KeyStore Can Fail

| Scenario | Issue |
|----------|-------|
| Emulators | Some images have incomplete KeyStore implementations |
| Older devices | KeyStore behavior varies by Android version and manufacturer |
| CI/CD | Running without full device context |
| NDK issues | JNI bridge may fail to initialize |

### Android Storage Options

| Features | Storage Method | Security | Reliability | Recommended For |
|----------|---------------|----------|-------------|-----------------|
| `mobile` | Android KeyStore | Highest (hardware-backed) | Variable | Production on tested devices |
| `mobile` + `file-store` | KeyStore with file fallback | High | Good | Production with safety net |
| `mobile` + `android-file` | Encrypted file only | Medium | Highest | Maximum compatibility |
| `mobile-sim` | KeyStore with file fallback | High | Good | Development/testing |

### Recommendations

**For development and emulator testing:**
```toml
dioxus-cookie = { version = "0.2", features = ["server", "mobile-sim"] }
```

**For production (reliability-focused):**
```toml
dioxus-cookie = { version = "0.2", features = ["server", "mobile", "android-file"] }
```
Skips KeyStore entirely. Slightly less secure but works on all devices.

**For production (security-focused, tested devices only):**
```toml
dioxus-cookie = { version = "0.2", features = ["server", "mobile"] }
```
Uses KeyStore only. Panics if KeyStore unavailable—only use after testing on target devices.

### Security Comparison

| Storage | Protection | Vulnerability |
|---------|------------|---------------|
| Android KeyStore | Hardware-backed on supported devices | None (keys cannot be extracted) |
| Encrypted File (`android-file`) | AES-256-GCM + app sandbox | Root access can extract data |

**Note:** File storage is protected by Android's app sandbox (other apps cannot access) and the filesystem is encrypted on Android 10+. The main risk is on rooted devices where an attacker has filesystem access.

### Expected Runtime Behavior

| Features | KeyStore Works | Result |
|----------|---------------|--------|
| `mobile` | Yes | Uses KeyStore |
| `mobile` | No | **Panics** |
| `mobile` + `file-store` | Yes | Uses KeyStore |
| `mobile` + `file-store` | No | Falls back to file |
| `mobile` + `android-file` | N/A | Skips KeyStore, uses file |
| `mobile-sim` | Yes | Uses KeyStore |
| `mobile-sim` | No | Falls back to file |

### Checking Storage Type

```rust
fn main() {
    dioxus_cookie::init();

    let storage = dioxus_cookie::get_storage_type();
    println!("Cookie storage: {storage}");
    // Android values: "android-keystore" or "android-file"

    dioxus::launch(App);
}
```

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

| Environment         | Why keychain/keystore fails   |
| ------------------- | ----------------------------- |
| iOS Simulator       | Missing keychain entitlements |
| Android Emulator    | KeyStore may be unavailable   |
| Linux without D-Bus | No Secret Service available   |
| CI/CD pipelines     | No desktop session            |
| Docker containers   | No keychain access            |

**Security limitations:**

- **DEBUG BUILDS ONLY** — Automatically disabled in release builds
- **Obfuscation, not protection** — AES-256-GCM encryption with a machine-derived key deters casual inspection but does not protect against local attackers with file access
- **Not for production** — Production apps must use real keychain storage

Use `mobile-sim` (which includes `file-store`) for simulator/emulator development, or add `file-store` directly for other fallback scenarios.

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
│  server   │      │  native   │      │   wasm    │
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
| `list_names()`              | List accessible cookie names (excludes HttpOnly on native).   |
| `get_storage_type()`        | Get the active storage backend.                               |

## Troubleshooting

**"No server context available"**
Cookie operations that set HTTP headers must run inside `#[server]` functions.

**Cookies not persisting on iOS Simulator**
Use the `mobile-sim` feature. The simulator lacks keychain entitlements, so cookies fall back to encrypted file storage.

**Cookies not persisting on Android**

1. Ensure `mobile` feature is enabled
2. The app must have proper NDK context (Dioxus Mobile provides this automatically)
3. Consider enabling `file-store` as fallback for emulators or devices where KeyStore fails
4. Note: Android support is experimental (via android-keyring crate)

**`get()` returns `None` for a cookie that was set**
The cookie is HttpOnly. This is intentional—use `get_internal()` only for session restoration.

## License

MIT
