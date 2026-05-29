use axum::{
    body::Body,
    extract::{State, Request},
    http::{header, StatusCode, Method},
    response::Response,
    routing::post,
    Json, Router,
};
use futures_util::StreamExt;
use serde::{Deserialize, Serialize};
use serde_json::json;
use sqlx::{
    sqlite::{SqliteConnectOptions, SqlitePoolOptions},
    SqlitePool,
};
use std::str::FromStr;
use std::sync::Arc;
use tokio_util::codec::{FramedRead, LinesCodec};
use tower_http::cors::{Any, CorsLayer};
use std::convert::Infallible;

#[derive(Deserialize, Debug)]
struct IncomingPayload {
    target: String,
    data: serde_json::Value,
    timestamp: i64,
    url: String,
}

#[derive(Deserialize, Debug)]
struct JsonRpcRequest {
    #[allow(dead_code)]
    jsonrpc: String,
    method: String,
    params: Option<serde_json::Value>,
    id: Option<serde_json::Value>,
}

#[derive(Serialize, Debug)]
struct JsonRpcResponse {
    jsonrpc: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    result: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    id: Option<serde_json::Value>,
}

struct AppState {
    pool: SqlitePool,
    debug: bool,
}

#[tokio::main]
async fn main() {
    let debug_mode = std::env::args().any(|arg| arg == "--debug");

    let options = SqliteConnectOptions::from_str("sqlite://saxo_data.db")
        .unwrap()
        .create_if_missing(true);

    let pool = SqlitePoolOptions::new()
        .max_connections(5)
        .connect_with(options)
        .await
        .expect("Failed to connect to SQLite");

    sqlx::query("PRAGMA journal_mode = WAL;")
        .execute(&pool)
        .await
        .unwrap();

    let state = Arc::new(AppState {
        pool,
        debug: debug_mode,
    });

    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods([Method::GET, Method::POST, Method::OPTIONS])
        .allow_headers([header::CONTENT_TYPE, header::AUTHORIZATION]);

    let app = Router::new()
        .route("/", post(handle_post))
        .route("/mcp", post(handle_mcp).options(handle_mcp_options))
        .layer(cors)
        .with_state(state);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:9876")
        .await
        .unwrap();
    eprintln!("Listening for extension pushes and MCP connections on http://localhost:9876 (Debug: {})", debug_mode);
    axum::serve(listener, app).await.unwrap();
}

async fn handle_mcp_options() -> StatusCode {
    StatusCode::OK
}

async fn handle_post(State(state): State<Arc<AppState>>, Json(payload): Json<IncomingPayload>) -> StatusCode {
    let target = payload.target;

    if target == "stockCharts" {
        let symbol = match payload.data["Snapshot"]["DisplayAndFormat"]["Symbol"].as_str() {
            Some(s) => s.replace(":", "_").replace(".", "_").replace("-", "_"),
            None => {
                eprintln!("stockCharts payload missing Symbol");
                return StatusCode::BAD_REQUEST;
            }
        };

        let create_table_query = format!(
            "CREATE TABLE IF NOT EXISTS {} (
                time TEXT PRIMARY KEY,
                open REAL,
                high REAL,
                low REAL,
                close REAL,
                volume REAL,
                interest REAL,
                market_trading_state TEXT
            )",
            symbol
        );

        if let Err(e) = sqlx::query(&create_table_query).execute(&state.pool).await {
            eprintln!("Failed to create stock table {}: {}", symbol, e);
            return StatusCode::INTERNAL_SERVER_ERROR;
        }

        if let Some(data_array) = payload.data["Snapshot"]["Data"].as_array() {
            for item in data_array {
                let time = item["Time"].as_str().unwrap_or("");
                let open = item["Open"].as_f64().unwrap_or(0.0);
                let high = item["High"].as_f64().unwrap_or(0.0);
                let low = item["Low"].as_f64().unwrap_or(0.0);
                let close = item["Close"].as_f64().unwrap_or(0.0);
                let volume = item["Volume"].as_f64().unwrap_or(0.0);
                let interest = item["Interest"].as_f64().unwrap_or(0.0);
                let state_val = item["MarketTradingState"].as_str().unwrap_or("");

                let insert_query = format!(
                    "INSERT OR REPLACE INTO {} (time, open, high, low, close, volume, interest, market_trading_state) VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
                    symbol
                );

                if let Err(e) = sqlx::query(&insert_query)
                    .bind(time)
                    .bind(open)
                    .bind(high)
                    .bind(low)
                    .bind(close)
                    .bind(volume)
                    .bind(interest)
                    .bind(state_val)
                    .execute(&state.pool)
                    .await {
                    eprintln!("Failed to insert stock data for {}: {}", symbol, e);
                }
            }
        }
        eprintln!("Successfully processed stockCharts for symbol: {}", symbol);
        return StatusCode::OK;
    }

    if target == "transactions" {
        let create_table_query = "CREATE TABLE IF NOT EXISTS transactions (
            bk_record_id INTEGER PRIMARY KEY,
            account_id TEXT,
            date TEXT,
            booked_amount REAL,
            currency TEXT,
            event TEXT,
            instrument_symbol TEXT,
            transaction_type TEXT,
            data TEXT
        )";

        if let Err(e) = sqlx::query(create_table_query).execute(&state.pool).await {
            eprintln!("Failed to create transactions table: {}", e);
            return StatusCode::INTERNAL_SERVER_ERROR;
        }

        sqlx::query("CREATE INDEX IF NOT EXISTS idx_transactions_date ON transactions (date)").execute(&state.pool).await.ok();
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_transactions_symbol ON transactions (instrument_symbol)").execute(&state.pool).await.ok();

        if let Some(data_array) = payload.data["Data"].as_array() {
            for item in data_array {
                let bk_record_id = item["BkRecordId"].as_i64().unwrap_or(0);
                let account_id = item["AccountId"].as_str().unwrap_or("");
                let date = item["Date"].as_str().unwrap_or("");
                let booked_amount = item["BookedAmount"].as_f64().unwrap_or(0.0);
                let currency = item["Currency"].as_str().unwrap_or("");
                let event = item["Event"].as_str().unwrap_or("");
                let instrument_symbol = item["Instrument"]["Symbol"].as_str().unwrap_or("");
                let transaction_type = item["TransactionType"].as_str().unwrap_or("");
                let raw_data = serde_json::to_string(item).unwrap_or_default();

                let insert_query = "INSERT OR IGNORE INTO transactions (bk_record_id, account_id, date, booked_amount, currency, event, instrument_symbol, transaction_type, data) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)";

                if let Err(e) = sqlx::query(insert_query)
                    .bind(bk_record_id)
                    .bind(account_id)
                    .bind(date)
                    .bind(booked_amount)
                    .bind(currency)
                    .bind(event)
                    .bind(instrument_symbol)
                    .bind(transaction_type)
                    .bind(raw_data)
                    .execute(&state.pool)
                    .await {
                    eprintln!("Failed to insert transaction {}: {}", bk_record_id, e);
                }
            }
        }
        eprintln!("Successfully processed transactions");
        return StatusCode::OK;
    }

    if target.is_empty() || !target.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
        eprintln!("Rejected payload with invalid target name: {}", target);
        return StatusCode::BAD_REQUEST;
    }

    let create_table_query = format!(
        "CREATE TABLE IF NOT EXISTS {} (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            timestamp INTEGER,
            url TEXT,
            data TEXT
        )",
        target
    );

    if let Err(e) = sqlx::query(&create_table_query).execute(&state.pool).await {
        eprintln!("Failed to create table {}: {}", target, e);
        return StatusCode::INTERNAL_SERVER_ERROR;
    }

    let create_index_query = format!(
        "CREATE INDEX IF NOT EXISTS idx_{}_timestamp ON {} (timestamp)",
        target, target
    );

    if let Err(e) = sqlx::query(&create_index_query).execute(&state.pool).await {
        eprintln!("Failed to create index for {}: {}", target, e);
        return StatusCode::INTERNAL_SERVER_ERROR;
    }

    let insert_query = format!(
        "INSERT INTO {} (timestamp, url, data) VALUES (?, ?, ?)",
        target
    );

    let data_str = serde_json::to_string(&payload.data).unwrap_or_default();

    match sqlx::query(&insert_query)
        .bind(payload.timestamp)
        .bind(payload.url)
        .bind(data_str)
        .execute(&state.pool)
        .await
    {
        Ok(_) => {
            eprintln!("Successfully saved data for target: {}", target);
            StatusCode::OK
        }
        Err(e) => {
            eprintln!("Failed to insert data into {}: {}", target, e);
            StatusCode::INTERNAL_SERVER_ERROR
        }
    }
}

async fn handle_mcp(State(state): State<Arc<AppState>>, request: Request) -> Response {
    let body = request.into_body();
    let stream = body.into_data_stream();
    
    let reader = tokio_util::io::StreamReader::new(stream.map(|res| {
        res.map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))
    }));
    
    let mut lines = FramedRead::new(reader, LinesCodec::new());
    let debug = state.debug;
    let pool = state.pool.clone();

    let output_stream = async_stream::stream! {
        while let Some(line_result) = lines.next().await {
            match line_result {
                Ok(line) => {
                    eprintln!("Received MCP Request: {}", line);
                    if let Ok(req) = serde_json::from_str::<JsonRpcRequest>(&line) {
                        if let Some(response) = process_mcp_request(req, &pool).await {
                            if let Ok(resp_json) = serde_json::to_string(&response) {
                                if debug {
                                    eprintln!("Sending MCP Response: {}", resp_json);
                                } else {
                                    eprintln!("Sending MCP Response ({} bytes)", resp_json.len());
                                }
                                yield Ok::<_, Infallible>(format!("{}\n", resp_json));
                            }
                        }
                    }
                }
                Err(_) => break,
            }
        }
    };

    Response::builder()
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from_stream(output_stream))
        .unwrap()
}

async fn process_mcp_request(req: JsonRpcRequest, pool: &SqlitePool) -> Option<JsonRpcResponse> {
    let is_notification = req.id.is_none();

    let response = match req.method.as_str() {
        "initialize" => {
            let client_version = req.params.as_ref()
                .and_then(|p| p["protocolVersion"].as_str())
                .unwrap_or("2024-11-05");
            
            Some(JsonRpcResponse {
                jsonrpc: "2.0".to_string(),
                result: Some(json!({
                    "protocolVersion": client_version,
                    "capabilities": {
                        "tools": {},
                        "resources": {}
                    },
                    "serverInfo": {
                        "name": "Saxo Grab Receiver",
                        "version": "0.1.0"
                    }
                })),
                error: None,
                id: req.id.clone(),
            })
        },
        "notifications/initialized" => {
            eprintln!("MCP Server Initialized");
            return None; 
        },
        "ping" => Some(JsonRpcResponse {
            jsonrpc: "2.0".to_string(),
            result: Some(json!({})),
            error: None,
            id: req.id.clone(),
        }),
        "tools/list" => Some(JsonRpcResponse {
            jsonrpc: "2.0".to_string(),
            result: Some(json!({
                "tools": [
                    {
                        "name": "query_stock_data",
                        "description": "Queries intercepted SaxoTrader data for standard targets: 'balances', 'netpositions', 'orders', 'news', 'transactions', 'earnings', 'watchlists'.",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "target": { "type": "string", "description": "Table name (e.g., balances)" },
                                "timestamp": { "type": "integer", "description": "Optional Unix timestamp to find the nearest record" }
                            },
                            "required": ["target"]
                        }
                    },
                    {
                        "name": "query_stock_history",
                        "description": "Queries historical price records for a specific stock symbol. Supports smart matching (e.g., 'SAN' matches 'SAN_xpar') and time range filtering.",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "symbol": { "type": "string", "description": "Stock symbol or table name (e.g., SAN, SAN_xpar)" },
                                "start_time": { "type": "string", "description": "Optional ISO 8601 time string for start of range" },
                                "end_time": { "type": "string", "description": "Optional ISO 8601 time string for end of range" }
                            },
                            "required": ["symbol"]
                        }
                    },
                    {
                        "name": "search_news",
                        "description": "Searches for news in the news table. You MUST provide either 'date' (YYYY-MM-DD), 'timestamp' (Unix ms), or both 'start_timestamp' and 'end_timestamp' (Unix ms).",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "date": { "type": "string", "description": "Exact date in YYYY-MM-DD format. Returns news for that day." },
                                "timestamp": { "type": "integer", "description": "Exact Unix timestamp in milliseconds. Returns news within +/- 24 hours." },
                                "start_timestamp": { "type": "integer", "description": "Start Unix timestamp in milliseconds." },
                                "end_timestamp": { "type": "integer", "description": "End Unix timestamp in milliseconds." }
                            }
                        }
                    },
                    {
                        "name": "search_transactions",
                        "description": "Searches for transactions. You can filter by 'date' (YYYY-MM-DD), a date range ('start_date' and 'end_date'), or 'symbol'.",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "date": { "type": "string", "description": "Exact date in YYYY-MM-DD format." },
                                "start_date": { "type": "string", "description": "Start date in YYYY-MM-DD format." },
                                "end_date": { "type": "string", "description": "End date in YYYY-MM-DD format." },
                                "symbol": { "type": "string", "description": "Instrument symbol (e.g., AMD:xnas)." }
                            }
                        }
                    }
                ]
            })),
            error: None,
            id: req.id.clone(),
        }),
        "tools/call" => {
            if let Some(params) = req.params {
                let tool_name = params["name"].as_str().unwrap_or("query_stock_data");
                let args = &params["arguments"];

                match tool_name {
                    "query_stock_data" => {
                        let target = args["target"].as_str().unwrap_or("");
                        let requested_ts = args["timestamp"].as_i64();

                        if target.is_empty() || !target.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
                            return Some(JsonRpcResponse {
                                jsonrpc: "2.0".to_string(),
                                result: None,
                                error: Some(json!({"code": -32602, "message": "Invalid target name"})),
                                id: req.id.clone(),
                            });
                        }

                        let nearest_record = match requested_ts {
                            None => {
                                let query = format!("SELECT timestamp, url, data FROM {} ORDER BY timestamp DESC LIMIT 1", target);
                                sqlx::query_as::<_, (i64, String, String)>(&query)
                                    .fetch_optional(pool)
                                    .await
                            }
                            Some(ts) => {
                                let query_le = format!("SELECT timestamp, url, data FROM {} WHERE timestamp <= ? ORDER BY timestamp DESC LIMIT 1", target);
                                let query_gt = format!("SELECT timestamp, url, data FROM {} WHERE timestamp > ? ORDER BY timestamp ASC LIMIT 1", target);
                                
                                let record_le = sqlx::query_as::<_, (i64, String, String)>(&query_le).bind(ts).fetch_optional(pool).await;
                                let record_gt = sqlx::query_as::<_, (i64, String, String)>(&query_gt).bind(ts).fetch_optional(pool).await;

                                match (record_le, record_gt) {
                                    (Ok(Some(r1)), Ok(Some(r2))) => {
                                        if (ts - r1.0).abs() <= (ts - r2.0).abs() {
                                            Ok(Some(r1))
                                        } else {
                                            Ok(Some(r2))
                                        }
                                    }
                                    (Ok(Some(r1)), _) => Ok(Some(r1)),
                                    (_, Ok(Some(r2))) => Ok(Some(r2)),
                                    (Err(e), _) => Err(e),
                                    (_, Err(e)) => Err(e),
                                    _ => Ok(None),
                                }
                            }
                        };

                        match nearest_record {
                            Ok(Some((ts, url, data))) => {
                                let prev_query = format!("SELECT timestamp FROM {} WHERE timestamp < ? ORDER BY timestamp DESC LIMIT 1", target);
                                let next_query = format!("SELECT timestamp FROM {} WHERE timestamp > ? ORDER BY timestamp ASC LIMIT 1", target);

                                let prev_ts: Option<(i64,)> = sqlx::query_as(&prev_query).bind(ts).fetch_optional(pool).await.unwrap_or(None);
                                let next_ts: Option<(i64,)> = sqlx::query_as(&next_query).bind(ts).fetch_optional(pool).await.unwrap_or(None);

                                let response_data = json!({
                                    "record": {
                                        "timestamp": ts,
                                        "url": url,
                                        "data": serde_json::from_str::<serde_json::Value>(&data).unwrap_or(json!(data))
                                    },
                                    "prev_timestamp": prev_ts.map(|r| r.0),
                                    "next_timestamp": next_ts.map(|r| r.0)
                                });

                                Some(JsonRpcResponse {
                                    jsonrpc: "2.0".to_string(),
                                    result: Some(json!({ "content": [{ "type": "text", "text": serde_json::to_string_pretty(&response_data).unwrap() }] })),
                                    error: None,
                                    id: req.id.clone(),
                                })
                            }
                            Ok(None) => Some(JsonRpcResponse {
                                jsonrpc: "2.0".to_string(),
                                result: Some(json!({ "content": [{ "type": "text", "text": "No records found" }] })),
                                error: None,
                                id: req.id.clone(),
                            }),
                            Err(e) => Some(JsonRpcResponse {
                                jsonrpc: "2.0".to_string(),
                                result: None,
                                error: Some(json!({"code": -32603, "message": format!("Error querying database: {}", e)})),
                                id: req.id.clone(),
                            })
                        }
                    },
                    "query_stock_history" => {
                        let symbol = args["symbol"].as_str().unwrap_or("");
                        let start_time = args["start_time"].as_str();
                        let end_time = args["end_time"].as_str();

                        if symbol.is_empty() {
                            return Some(JsonRpcResponse {
                                jsonrpc: "2.0".to_string(),
                                result: None,
                                error: Some(json!({"code": -32602, "message": "Missing symbol"})),
                                id: req.id.clone(),
                            });
                        }

                        // Smart matching
                        let matching_tables: Vec<(String,)> = match sqlx::query_as("SELECT name FROM sqlite_master WHERE type='table' AND (name = ? OR name LIKE ?)")
                            .bind(symbol)
                            .bind(format!("{}_%", symbol))
                            .fetch_all(pool)
                            .await {
                                Ok(t) => t,
                                Err(e) => return Some(JsonRpcResponse {
                                    jsonrpc: "2.0".to_string(),
                                    result: None,
                                    error: Some(json!({"code": -32603, "message": format!("Error searching for tables: {}", e)})),
                                    id: req.id.clone(),
                                })
                            };
                        
                        let resolved_table = match matching_tables.len() {
                            0 => return Some(JsonRpcResponse {
                                jsonrpc: "2.0".to_string(),
                                result: Some(json!({ "content": [{ "type": "text", "text": format!("No data found for symbol: {}", symbol) }] })),
                                error: None,
                                id: req.id.clone(),
                            }),
                            1 => matching_tables[0].0.clone(),
                            _ => {
                                let table_names: Vec<String> = matching_tables.into_iter().map(|t| t.0).collect();
                                return Some(JsonRpcResponse {
                                    jsonrpc: "2.0".to_string(),
                                    result: Some(json!({ "content": [{ "type": "text", "text": format!("Multiple markets found for {}. Please specify one: {}", symbol, table_names.join(", ")) }] })),
                                    error: None,
                                    id: req.id.clone(),
                                });
                            }
                        };

                        // Build query
                        let mut query_str = format!("SELECT time, open, high, low, close, volume, interest, market_trading_state FROM {} WHERE 1=1", resolved_table);
                        if start_time.is_some() {
                            query_str.push_str(" AND time >= ?");
                        }
                        if end_time.is_some() {
                            query_str.push_str(" AND time <= ?");
                        }
                        query_str.push_str(" ORDER BY time ASC");

                        let mut query = sqlx::query_as::<_, (String, f64, f64, f64, f64, f64, f64, String)>(&query_str);
                        if let Some(st) = start_time {
                            query = query.bind(st);
                        }
                        if let Some(et) = end_time {
                            query = query.bind(et);
                        }

                        match query.fetch_all(pool).await {
                            Ok(records) => {
                                let result_data: Vec<serde_json::Value> = records.into_iter().map(|r| json!({
                                    "time": r.0,
                                    "open": r.1,
                                    "high": r.2,
                                    "low": r.3,
                                    "close": r.4,
                                    "volume": r.5,
                                    "interest": r.6,
                                    "market_trading_state": r.7
                                })).collect();
                                let response_data = json!({
                                    "symbol": symbol,
                                    "table": resolved_table,
                                    "history": result_data,
                                    "count": result_data.len()
                                });
                                Some(JsonRpcResponse {
                                    jsonrpc: "2.0".to_string(),
                                    result: Some(json!({ "content": [{ "type": "text", "text": serde_json::to_string_pretty(&response_data).unwrap() }] })),
                                    error: None,
                                    id: req.id.clone(),
                                })
                            }
                            Err(e) => Some(JsonRpcResponse {
                                jsonrpc: "2.0".to_string(),
                                result: None,
                                error: Some(json!({"code": -32603, "message": format!("Error querying history: {}", e)})),
                                id: req.id.clone(),
                            })
                        }
                    },
                    "search_news" => {
                        let date = args["date"].as_str();
                        let ts = args["timestamp"].as_i64();
                        let start_ts = args["start_timestamp"].as_i64();
                        let end_ts = args["end_timestamp"].as_i64();

                        let (q_start, q_end) = if let Some(d) = date {
                            let base_ts: i64 = sqlx::query_scalar("SELECT CAST(unixepoch(?) * 1000 AS INTEGER)")
                                .bind(d)
                                .fetch_one(pool)
                                .await
                                .unwrap_or(0);
                            if base_ts == 0 {
                                return Some(JsonRpcResponse {
                                    jsonrpc: "2.0".to_string(),
                                    result: None,
                                    error: Some(json!({"code": -32602, "message": "Invalid date format. Use YYYY-MM-DD"})),
                                    id: req.id.clone(),
                                });
                            }
                            (base_ts, base_ts + 86_400_000)
                        } else if let Some(t) = ts {
                            (t - 86_400_000, t + 86_400_000)
                        } else if let (Some(s), Some(e)) = (start_ts, end_ts) {
                            (s, e)
                        } else {
                            return Some(JsonRpcResponse {
                                jsonrpc: "2.0".to_string(),
                                result: None,
                                error: Some(json!({"code": -32602, "message": "Missing time parameters. Provide date, timestamp, or start_timestamp/end_timestamp."})),
                                id: req.id.clone(),
                            });
                        };

                        // Check if news table exists
                        let table_exists: (i32,) = sqlx::query_as("SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='news'")
                            .fetch_one(pool)
                            .await
                            .unwrap_or((0,));

                        if table_exists.0 == 0 {
                            return Some(JsonRpcResponse {
                                jsonrpc: "2.0".to_string(),
                                result: Some(json!({ "content": [{ "type": "text", "text": "{\"news\": [], \"message\": \"News table does not exist yet. No news data has been intercepted.\"}" }] })),
                                error: None,
                                id: req.id.clone(),
                            });
                        }

                        let query_str = "SELECT timestamp, url, data FROM news WHERE timestamp >= ? AND timestamp <= ? ORDER BY timestamp ASC LIMIT 100";
                        match sqlx::query_as::<_, (i64, String, String)>(query_str)
                            .bind(q_start)
                            .bind(q_end)
                            .fetch_all(pool)
                            .await
                        {
                            Ok(records) => {
                                let result_data: Vec<serde_json::Value> = records.into_iter().map(|r| json!({
                                    "timestamp": r.0,
                                    "url": r.1,
                                    "data": serde_json::from_str::<serde_json::Value>(&r.2).unwrap_or(json!(r.2))
                                })).collect();
                                let response_data = json!({
                                    "news": result_data,
                                    "count": result_data.len(),
                                    "range": { "start": q_start, "end": q_end }
                                });
                                Some(JsonRpcResponse {
                                    jsonrpc: "2.0".to_string(),
                                    result: Some(json!({ "content": [{ "type": "text", "text": serde_json::to_string_pretty(&response_data).unwrap() }] })),
                                    error: None,
                                    id: req.id.clone(),
                                })
                            }
                            Err(e) => Some(JsonRpcResponse {
                                jsonrpc: "2.0".to_string(),
                                result: None,
                                error: Some(json!({"code": -32603, "message": format!("Error querying news: {}", e)})),
                                id: req.id.clone(),
                            })
                        }
                    },
                    "search_transactions" => {
                        let date = args["date"].as_str();
                        let start_date = args["start_date"].as_str();
                        let end_date = args["end_date"].as_str();
                        let symbol = args["symbol"].as_str();

                        // Check if transactions table exists
                        let table_exists: (i32,) = sqlx::query_as("SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='transactions'")
                            .fetch_one(pool)
                            .await
                            .unwrap_or((0,));

                        if table_exists.0 == 0 {
                            return Some(JsonRpcResponse {
                                jsonrpc: "2.0".to_string(),
                                result: Some(json!({ "content": [{ "type": "text", "text": "{\"transactions\": [], \"message\": \"Transactions table does not exist yet.\"}" }] })),
                                error: None,
                                id: req.id.clone(),
                            });
                        }

                        let mut query_str = "SELECT data FROM transactions WHERE 1=1".to_string();
                        if date.is_some() {
                            query_str.push_str(" AND date = ?");
                        }
                        if start_date.is_some() {
                            query_str.push_str(" AND date >= ?");
                        }
                        if end_date.is_some() {
                            query_str.push_str(" AND date <= ?");
                        }
                        if symbol.is_some() {
                            query_str.push_str(" AND instrument_symbol = ?");
                        }
                        query_str.push_str(" ORDER BY date DESC LIMIT 100");

                        let mut query = sqlx::query_scalar::<_, String>(&query_str);
                        if let Some(d) = date {
                            query = query.bind(d);
                        }
                        if let Some(sd) = start_date {
                            query = query.bind(sd);
                        }
                        if let Some(ed) = end_date {
                            query = query.bind(ed);
                        }
                        if let Some(s) = symbol {
                            query = query.bind(s);
                        }

                        match query.fetch_all(pool).await {
                            Ok(records) => {
                                let result_data: Vec<serde_json::Value> = records.into_iter().map(|r| {
                                    serde_json::from_str::<serde_json::Value>(&r).unwrap_or(json!(r))
                                }).collect();
                                let response_data = json!({
                                    "transactions": result_data,
                                    "count": result_data.len()
                                });
                                Some(JsonRpcResponse {
                                    jsonrpc: "2.0".to_string(),
                                    result: Some(json!({ "content": [{ "type": "text", "text": serde_json::to_string_pretty(&response_data).unwrap() }] })),
                                    error: None,
                                    id: req.id.clone(),
                                })
                            }
                            Err(e) => Some(JsonRpcResponse {
                                jsonrpc: "2.0".to_string(),
                                result: None,
                                error: Some(json!({"code": -32603, "message": format!("Error querying transactions: {}", e)})),
                                id: req.id.clone(),
                            })
                        }
                    },
                    _ => Some(JsonRpcResponse {
                        jsonrpc: "2.0".to_string(),
                        result: None,
                        error: Some(json!({"code": -32601, "message": format!("Method '{}' not implemented", tool_name)})),
                        id: req.id.clone(),
                    }),
                }
            } else {
                Some(JsonRpcResponse {
                    jsonrpc: "2.0".to_string(),
                    result: None,
                    error: Some(json!({"code": -32602, "message": "Missing parameters"})),
                    id: req.id.clone(),
                })
            }
        },
        "resources/list" => Some(JsonRpcResponse {
            jsonrpc: "2.0".to_string(),
            result: Some(json!({
                "resources": [
                    {
                        "uri": "schema://stock_data",
                        "name": "Saxo Data Schema Info",
                        "description": "Information about how the stock data is structured and where to find it.",
                        "mimeType": "text/plain"
                    }
                ]
            })),
            error: None,
            id: req.id.clone(),
        }),
        "resources/read" => {
            if let Some(params) = req.params {
                if params["uri"] == "schema://stock_data" {
                    Some(JsonRpcResponse {
                        jsonrpc: "2.0".to_string(),
                        result: Some(json!({
                            "contents": [
                                {
                                    "uri": "schema://stock_data",
                                    "mimeType": "text/plain",
                                    "text": "Saxo data is stored in SQLite (saxo_data.db). 
- Standard targets (balances, netpositions, etc.) have columns: id, timestamp, url, data.
- Stock chart tables (named by symbol) have columns: time (PK), open, high, low, close, volume, interest, market_trading_state."
                                }
                            ]
                        })),
                        error: None,
                        id: req.id.clone(),
                    })
                } else {
                    Some(JsonRpcResponse {
                        jsonrpc: "2.0".to_string(),
                        result: None,
                        error: Some(json!({"code": -32602, "message": "Resource not found"})),
                        id: req.id.clone(),
                    })
                }
            } else {
                Some(JsonRpcResponse {
                    jsonrpc: "2.0".to_string(),
                    result: None,
                    error: Some(json!({"code": -32602, "message": "Missing parameters"})),
                    id: req.id.clone(),
                })
            }
        },
        _ => Some(JsonRpcResponse {
            jsonrpc: "2.0".to_string(),
            result: None,
            error: Some(json!({ "code": -32601, "message": "Method not implemented" })),
            id: req.id.clone(),
        }),
    };

    if is_notification {
        None
    } else {
        response
    }
}
