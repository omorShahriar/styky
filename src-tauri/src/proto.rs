//! Wire protocol of the local API, shared by the app (server) and the `styky` CLI / MCP client.
//!
//! Transport: the named pipe `\\.\pipe\styky-<username>` (byte mode), one JSON object per line.
//! Request  `{"id": 1, "method": "add", "params": {...}}`
//! Response `{"id": 1, "result": ...}` or `{"id": 1, "error": {"code": "requires_pro", "message": "..."}}`

use serde_json::{Value, json};

pub const NOT_RUNNING: &str = "Styky isn't running. Start it from the Start menu, then try again.";

/// `\\.\pipe\styky-<username>`: one pipe per Windows user.
pub fn pipe_name() -> String {
    let user = std::env::var("USERNAME").unwrap_or_else(|_| "user".into());
    let user: String = user.chars().filter(|c| c.is_alphanumeric() || matches!(c, '-' | '_' | '.')).collect();
    format!(r"\\.\pipe\styky-{user}")
}

#[derive(Debug, Clone, PartialEq)]
pub struct ApiError {
    pub code: String,
    pub message: String,
}

impl ApiError {
    pub fn new(code: &str, message: impl Into<String>) -> Self {
        Self { code: code.into(), message: message.into() }
    }
}

impl std::fmt::Display for ApiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.message)
    }
}

#[allow(dead_code)] // server side only; the CLI shares this file
pub fn ok_response(id: &Value, result: Value) -> String {
    json!({ "id": id, "result": result }).to_string()
}

#[allow(dead_code)]
pub fn err_response(id: &Value, e: &ApiError) -> String {
    json!({ "id": id, "error": { "code": e.code, "message": e.message } }).to_string()
}

/// Send one request over the pipe `name` and wait for the answer.
#[cfg(windows)]
pub fn call_on(name: &str, method: &str, params: Value) -> Result<Value, ApiError> {
    use std::io::{BufRead, BufReader, Write};
    let mut tries = 0;
    let pipe = loop {
        match std::fs::OpenOptions::new().read(true).write(true).open(name) {
            Ok(f) => break f,
            // ERROR_PIPE_BUSY: all instances are in use for a moment.
            Err(e) if e.raw_os_error() == Some(231) && tries < 20 => {
                tries += 1;
                std::thread::sleep(std::time::Duration::from_millis(50));
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Err(ApiError::new("not_running", NOT_RUNNING)),
            Err(e) => return Err(ApiError::new("io", format!("cannot reach Styky: {e}"))),
        }
    };
    let io = |e: std::io::Error| ApiError::new("io", format!("connection to Styky failed: {e}"));
    let mut w = &pipe;
    let req = json!({ "id": 1, "method": method, "params": params });
    w.write_all(format!("{req}\n").as_bytes()).map_err(io)?;
    w.flush().map_err(io)?;
    let mut line = String::new();
    BufReader::new(&pipe).read_line(&mut line).map_err(io)?;
    let v: Value = serde_json::from_str(&line).map_err(|e| ApiError::new("protocol", format!("bad reply from Styky: {e}")))?;
    match v.get("error") {
        Some(e) if !e.is_null() => Err(ApiError::new(
            e["code"].as_str().unwrap_or("error"),
            e["message"].as_str().unwrap_or("unknown error"),
        )),
        _ => Ok(v.get("result").cloned().unwrap_or(Value::Null)),
    }
}

#[cfg(not(windows))]
pub fn call_on(_name: &str, _method: &str, _params: Value) -> Result<Value, ApiError> {
    Err(ApiError::new("unsupported", "Styky only runs on Windows"))
}

/// Call the running app's API.
pub fn call(method: &str, params: Value) -> Result<Value, ApiError> {
    call_on(&pipe_name(), method, params)
}
