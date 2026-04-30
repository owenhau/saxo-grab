# Changelog

All notable changes to this project will be documented in this file.

## [1.0.2] - 2026-04-30

### Changed
- Improved downloaded filenames to be more descriptive (e.g., \`balances.json\`, \`netpositions.json\`).

## [1.0.1] - 2026-04-30

### Changed
- Removed timestamp suffix from downloaded JSON filenames.

## [1.0.0] - 2026-04-30

### Added
- Initial release of the SaxoTrader JSON Grabber extension.
- Network interception for 4 key SaxoTrader API endpoints:
    - Balances Subscriptions
    - Net Positions Subscriptions
    - Orders Subscriptions
    - News Subscriptions
- Popup UI for monitoring capture status and triggering downloads.
- Background service worker for handling file downloads.
- Local storage persistence for captured JSON data.
- Documentation: `AGENTS.md`, `README.md`, and `CHANGELOG.md`.
