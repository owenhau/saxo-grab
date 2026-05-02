# SaxoTrader JSON Grabber & MCP Receiver

A system consisting of a Chrome Extension and a Rust-based receiver to intercept, store, and provide AI access to SaxoTrader portfolio data.

## Features

- **Autonomous Interception**: Automatically detects and captures JSON responses from key SaxoTrader API endpoints.
- **Extended Monitoring**:
    - Balances, Net Positions, Orders, News.
    - **Transactions** and **Earnings** (with historical filtering for `FromDate < 2025-02-27`).
    - **Watchlists**.
- **Live Status Dashboard**: A button-free popup UI showing real-time monitoring status (Waiting, Updating, Success, or Error).
- **Rust Receiver**: A high-performance backend that receives data from the extension and stores it in a partitioned SQLite database.
- **Model Context Protocol (MCP)**: Acts as a "Streamable HTTP" MCP server, allowing AI agents to query portfolio data and schema information.

## Architecture

- **Extension (`chrome/`)**:
    - **Manifest V3**: Modern extension architecture.
    - **Injection**: Overrides `fetch`/`XHR` in the main world to intercept data without extra network overhead.
    - **Data Push**: Automatically forwards intercepted JSON to the local receiver.
- **Receiver (`receiver/`)**:
    - **Axum**: Asynchronous web server handling data pushes and MCP connections.
    - **SQLite (sqlx)**: Partitioned storage with tables per target and indexed timestamps for fast querying.
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
4.  **AI Integration**: Point your MCP-compatible AI agent to `http://127.0.0.1:9876/mcp` to query the stored data using the `query_stock_data` tool.

## Security Note

This system is for personal data retrieval and research. It communicates only with `localhost:9876`. Data is stored locally in `saxo_data.db`.
