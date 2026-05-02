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

    let result = match req.method.as_str() {
        "initialize" => {
            let client_version = req.params.as_ref()
                .and_then(|p| p["protocolVersion"].as_str())
                .unwrap_or("2024-11-05");
            
            Some(json!({
                "protocolVersion": client_version,
                "capabilities": {
                    "tools": {},
                    "resources": {}
                },
                "serverInfo": {
                    "name": "Saxo Grab Receiver",
                    "version": "0.1.0"
                }
            }))
        },
        "notifications/initialized" => {
            eprintln!("MCP Server Initialized");
            return None; 
        },
        "ping" => Some(json!({})),
        "tools/list" => Some(json!({
            "tools": [
                {
                    "name": "query_stock_data",
                    "description": "Queries intercepted SaxoTrader data from the local SQLite database. Tables are named after targets: 'balances', 'netpositions', 'orders', 'news', 'transactions', 'earnings', 'watchlists'.",
                    "inputSchema": {
                        "type": "object",
                        "properties": {
                            "target": { "type": "string", "description": "Table name (e.g., balances, orders)" },
                            "start_timestamp": { "type": "integer", "description": "Optional start Unix timestamp" },
                            "end_timestamp": { "type": "integer", "description": "Optional end Unix timestamp" },
                            "limit": { "type": "integer", "default": 100 }
                        },
                        "required": ["target"]
                    }
                }
            ]
        })),
        "tools/call" => {
            if let Some(params) = req.params {
                let args = &params["arguments"];
                let target = args["target"].as_str().unwrap_or("");
                let start = args["start_timestamp"].as_i64().unwrap_or(0);
                let end = args["end_timestamp"].as_i64().unwrap_or(i64::MAX);
                let limit = args["limit"].as_i64().unwrap_or(100);

                if target.is_empty() || !target.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
                    return Some(JsonRpcResponse {
                        jsonrpc: "2.0".to_string(),
                        result: None,
                        error: Some(json!({"code": -32602, "message": "Invalid target name"})),
                        id: req.id,
                    });
                }

                let query = format!(
                    "SELECT timestamp, url, data FROM {} WHERE timestamp >= ? AND timestamp <= ? ORDER BY timestamp DESC LIMIT ?",
                    target
                );

                match sqlx::query_as::<_, (i64, String, String)>(&query)
                    .bind(start)
                    .bind(end)
                    .bind(limit)
                    .fetch_all(pool)
                    .await
                {
                    Ok(rows) => {
                        let content_items = rows.into_iter().map(|(ts, url, data)| {
                            json!({
                                "timestamp": ts,
                                "url": url,
                                "data": serde_json::from_str::<serde_json::Value>(&data).unwrap_or(json!(data))
                            })
                        }).collect::<Vec<_>>();
                        Some(json!({ "content": [{ "type": "text", "text": serde_json::to_string_pretty(&content_items).unwrap() }] }))
                    }
                    Err(e) => Some(json!({ "content": [{ "type": "text", "text": format!("Error querying database: {}", e) }], "isError": true }))
                }
            } else {
                Some(json!({"code": -32602, "message": "Missing parameters"}))
            }
        },
        "resources/list" => Some(json!({
            "resources": [
                {
                    "uri": "schema://stock_data",
                    "name": "Saxo Data Schema Info",
                    "description": "Information about how the stock data is structured and where to find it.",
                    "mimeType": "text/plain"
                }
            ]
        })),
        "resources/read" => {
            if let Some(params) = req.params {
                if params["uri"] == "schema://stock_data" {
                    Some(json!({
                        "contents": [
                            {
                                "uri": "schema://stock_data",
                                "mimeType": "text/plain",
                                "text": "Saxo data is stored in SQLite (saxo_data.db). 
Each target (balances, netpositions, orders, news, transactions, earnings, watchlists) has its own table.
Table columns:
- id: INTEGER PRIMARY KEY
- timestamp: INTEGER (Unix ms, indexed, sorted DESC by default)
- url: TEXT (source API URL)
- data: TEXT (JSON payload)"
                            }
                        ]
                    }))
                } else {
                    Some(json!({"code": -32602, "message": "Resource not found"}))
                }
            } else {
                Some(json!({"code": -32602, "message": "Missing parameters"}))
            }
        },
        _ => Some(json!({ "code": -32601, "message": "Method not implemented" })),
    };

    if is_notification {
        None
    } else {
        Some(JsonRpcResponse {
            jsonrpc: "2.0".to_string(),
            result,
            error: None,
            id: req.id,
        })
    }
}
