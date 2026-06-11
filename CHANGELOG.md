# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- System tray icon with multiplatform support (Linux/Windows/macOS)
- QR code in web UI for mobile access
- `/api/local-ip` endpoint for network IP discovery
- OBS plugin with full WebSocket 5.x integration
- Volume master plugin with per-app control (Linux/Windows)
- Widget system with 10+ widget types

### Changed
- Simplified tray menu to: Status, Open in Browser, Exit
- QR code now uses real local IP from API instead of `window.location.origin`

### Fixed
- Linux event loop creation on background thread
- Menu event handling with `ControlFlow::Poll`
- QR code generation overflow panic

## [0.1.0] - 2026-06-11

### Added
- Initial release
- Plugin system with libloading
- Web UI with Preact + TypeScript
- System monitor, timer, key simulator plugins
- Virtual StreamDeck device
- Profile management
- WebSocket real-time events
