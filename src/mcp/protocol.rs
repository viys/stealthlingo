//! JSON-RPC 2.0 messages as used by the Model Context Protocol.

use serde_json::{json, Value};

pub const PARSE_ERROR: i64 = -32700;
pub const INVALID_REQUEST: i64 = -32600;
pub const METHOD_NOT_FOUND: i64 = -32601;
pub const INVALID_PARAMS: i64 = -32602;
pub const INTERNAL_ERROR: i64 = -32603;

/// Protocol versions this server speaks, newest first.
pub const SUPPORTED_VERSIONS: [&str; 2] = ["2025-06-18", "2025-03-26"];

/// The version to use with a client that asked for `requested`.
pub fn negotiate(requested: Option<&str>) -> &'static str {
    SUPPORTED_VERSIONS
        .into_iter()
        .find(|v| Some(*v) == requested)
        .unwrap_or(SUPPORTED_VERSIONS[0])
}

/// Whether tools may declare `outputSchema` and return `structuredContent`.
pub fn supports_structured_output(version: &str) -> bool {
    version >= "2025-06-18"
}

#[derive(Debug, Clone, PartialEq)]
pub struct RpcError {
    pub code: i64,
    pub message: String,
}

impl RpcError {
    pub fn new(code: i64, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
        }
    }

    pub fn invalid_params(message: impl Into<String>) -> Self {
        Self::new(INVALID_PARAMS, message)
    }
}

/// A message from the client.
#[derive(Debug, Clone, PartialEq)]
pub enum Incoming {
    Request {
        id: Value,
        method: String,
        params: Value,
    },
    Notification {
        method: String,
    },
    /// A response to a request we never send, or anything else to ignore.
    Other,
}

/// Parses one line. `Err` carries the response to send back.
pub fn parse(line: &str) -> Result<Incoming, Value> {
    let value: Value = serde_json::from_str(line)
        .map_err(|e| error(Value::Null, PARSE_ERROR, &format!("invalid JSON: {e}")))?;
    let Value::Object(message) = value else {
        return Err(error(
            Value::Null,
            INVALID_REQUEST,
            "expected a single JSON-RPC message object",
        ));
    };
    let id = message.get("id").cloned();
    if message.get("jsonrpc").and_then(Value::as_str) != Some("2.0") {
        return Err(error(
            id.unwrap_or(Value::Null),
            INVALID_REQUEST,
            "\"jsonrpc\" must be \"2.0\"",
        ));
    }
    let Some(method) = message.get("method") else {
        return Ok(Incoming::Other);
    };
    let Some(method) = method.as_str().map(str::to_string) else {
        return Err(error(
            id.unwrap_or(Value::Null),
            INVALID_REQUEST,
            "\"method\" must be a string",
        ));
    };
    let params = message.get("params").cloned().unwrap_or(Value::Null);
    Ok(match id {
        Some(id) => Incoming::Request { id, method, params },
        None => Incoming::Notification { method },
    })
}

pub fn result(id: Value, result: Value) -> Value {
    json!({ "jsonrpc": "2.0", "id": id, "result": result })
}

pub fn error(id: Value, code: i64, message: &str) -> Value {
    json!({ "jsonrpc": "2.0", "id": id, "error": { "code": code, "message": message } })
}
