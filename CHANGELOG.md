# Changelog

All notable changes to this project will be documented in this file.

## [2.0.0] - 2026-05-01

### Added
- **Rust Receiver**: Introduced a backend server to store intercepted data in SQLite.
- **MCP Server**: Integrated a Streamable HTTP MCP server for AI agent data access.
- **New Targets**: Added monitoring for Transactions, Earnings, and Watchlists.
- **Historical Filtering**: Implemented `FromDate` filtering for Transactions and Earnings.
- **Status Dashboard**: Redesigned the extension popup into a live monitoring dashboard.
- **Debug Mode**: Added `--debug` flag to the receiver for controlled verbosity.

### Changed
- **Reorganization**: Moved extension files into a dedicated `chrome/` directory.
- **Data Flow**: Switched from manual downloads to autonomous pushing from the extension to the receiver.
- **Security**: Updated manifest permissions for local server communication.

## [1.0.3] - 2026-04-30

### Added
- Added "Download All" button to download all captured JSON files at once.

## [1.0.0] - 2026-04-30
- Initial release of the SaxoTrader JSON Grabber extension.
