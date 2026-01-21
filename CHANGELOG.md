# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

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
