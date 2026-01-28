# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.2.0] - 2025-01-27

### Added

- Android KeyStore support via `android-keyring` crate (experimental)
- `android-file` feature: Forces encrypted file storage on Android, skipping KeyStore for maximum reliability
- Android-specific error detection for automatic file-store fallback
- Comprehensive Android platform documentation with storage options and recommendations

### Fixed

- Android KeyStore init failure now correctly propagates to force file storage mode

### Changed

- `mobile` feature now includes both iOS and Android support
- `get_storage_type()` returns `"android-keystore"` or `"android-file"` on Android for better debugging
- Updated documentation to clarify HttpOnly cookie use case

## [0.1.0] - 2025-01-21

### Added

- Initial release
- Cross-platform cookie API (`get`, `set`, `clear`, `list_names`)
- Server-side cookie handling via HTTP headers (`server` feature)
- Desktop keyring storage via macOS Keychain, Windows Credential Manager, Linux Secret Service (`desktop` feature)
- Mobile keyring storage via iOS Keychain and Android Keystore (`mobile` feature)
- Browser support via `document.cookie` (default, no feature required)
- Encrypted file fallback for iOS Simulator and environments without keychain access (`file-store` feature)
- HttpOnly cookie enforcement on native platforms
- RFC 6265 compliant cookie parsing and expiration
- Automatic cookie transmission with Dioxus server function calls
