# SaxoTrader JSON Grabber & MCP Receiver

A system consisting of a Chrome Extension and a Rust-based receiver to intercept, store, and provide AI access to SaxoTrader portfolio data.

## Features

- **Autonomous Interception**: Automatically detects and captures JSON responses from key SaxoTrader API endpoints.
- **Extended Monitoring**:
    - Balances, Net Positions, Orders, News, Watchlists.
    - **Transactions** and **Earnings** (Full historical retrieval, no date limits).
    - **Stock Charts** (Candlestick data with symbol-based storage).
- **Live Status Dashboard**: A button-free popup UI showing real-time monitoring status (Waiting, Updating, Success, or Error).
- **Rust Receiver**: A high-performance backend that receives data from the extension and stores it in a partitioned SQLite database with idempotency checks.
- **Model Context Protocol (MCP)**: Acts as a "Streamable HTTP" MCP server, allowing AI agents to query portfolio data, transaction history, and stock charts.

## Architecture

- **Extension (`chrome/`)**:
    - **Manifest V3**: Modern extension architecture.
    - **Injection**: Overrides `fetch`/`XHR` in the main world to intercept data without extra network overhead.
    - **Data Push**: Automatically forwards intercepted JSON to the local receiver.
- **Receiver (`receiver/`)**:
    - **Axum**: Asynchronous web server handling data pushes and MCP connections.
    - **SQLite (sqlx)**: Partitioned storage. Standard data is stored by target, while **Transactions** use a structured schema and **Stock Charts** are stored in symbol-specific tables.
    - **MCP Server**: Implements bidirectional NDJSON streaming for AI agent integration.

## Installation

### 1. Chrome Extension
1.  Open Google Chrome and navigate to `chrome://extensions/`.
2.  Enable **Developer mode**.
3.  Click **Load unpacked** and select the `chrome/` directory.

### 2. Rust Receiver
1.  Ensure you have [Rust](https://www.rust-lang.org/) installed.
2.  Navigate to the `receiver` directory.
3.  Run the server:
    ```bash
    cargo run
    ```
    *(Use `cargo run -- --debug` for verbose JSON logging)*

## How to Use

1.  Start the **Rust Receiver**.
2.  Navigate to SaxoTrader. The extension will automatically start pushing data to the receiver.
3.  Check the extension popup to monitor capture status.
4.  **AI Integration**: Point your MCP-compatible AI agent to `http://127.0.0.1:9876/mcp` to query the stored data using:
    - `query_stock_data`: General purpose target retrieval.
    - `search_transactions`: Structured transaction history search.
    - `query_stock_history`: Historical price records for specific symbols.

## Security Note

This system is for personal data retrieval and research. It communicates only with `localhost:9876`. Data is stored locally in `saxo_data.db`.
