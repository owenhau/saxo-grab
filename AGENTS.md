# Saxo Grabber Implementation Guide

### System Overview
Saxo Grabber is a dual-component system designed for automated retrieval and AI-augmented analysis of trading data.

### Component 1: The Interceptor (Chrome Extension)
Located in `chrome/`, this component handles the data acquisition layer.

- **`interceptor.js`**: Injected main-world script that patches browser primitives. It applies business logic filters (like historical date ranges) before signaling the extension.
- **`content.js`**: Orchestrates status reporting to the popup and executes the `POST` push to the local backend.
- **`popup.html/js/css`**: A reactive dashboard that displays the current health and activity of the 6 monitored target streams.

### Component 2: The Receiver (Rust Backend)
Located in `receiver/`, this component handles persistence and the interface for AI agents.

- **Storage Layer**: Uses SQLite with **WAL mode** for performance. Tables are created dynamically per target (`balances`, `netpositions`, `orders`, `news`, `transactions`, `earnings`, `watchlists`).
- **Index Strategy**: Every table is indexed on the `timestamp` column for high-performance temporal queries.
- **MCP Interface**: A bidirectional NDJSON stream over HTTP. 
    - **Resource**: `schema://stock_data` for discovery.
    - **Tool**: `query_stock_data` for surgical data retrieval.

### Deployment & Maintenance
1. **Adding Targets**: Update `interceptor.js` (logic), `content.js` (mapping), and `popup.html` (display). The receiver handles new tables automatically.
2. **AI Configuration**: Ensure the AI agent is configured with the `Streamable HTTP` transport pointing to the `/mcp` endpoint.
3. **Database Maintenance**: The `saxo_data.db` file is standard SQLite and can be queried or backed up using any standard tool.
