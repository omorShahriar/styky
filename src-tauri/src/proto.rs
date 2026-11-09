//! Wire protocol of the local API, shared by the app (server) and the `styky` CLI / MCP client.
//!
//! Transport: the named pipe `\\.\pipe\styky-<username>` (byte mode), one JSON object per line.
//! Request  `{"id": 1, "method": "add", "params": {...}}`
//! Response `{"id": 1, "result": ...}` or `{"id": 1, "error": {"code": "requires_pro", "message": "..."}}`

use serde_json::{Value, json};

pub const NOT_RUNNING: &str = "Styky isn't running. Start it from the Start menu, then try again.";

/// Where a Pro feature is explained. Every `requires_pro` message ends with it.
pub const PRO_URL: &str = "https://styky.smallhingelabs.com/pro";

/// Machine-readable error code for "this request uses a Pro feature".
pub const REQUIRES_PRO: &str = "requires_pro";

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
    /// Extra machine-readable detail (for `requires_pro`: `{"feature": "...", "url": "..."}`).
    pub data: Option<Value>,
}

impl ApiError {
    pub fn new(code: &str, message: impl Into<String>) -> Self {
        Self { code: code.into(), message: message.into(), data: None }
    }

    /// A request used a Pro feature on a Free install. Only that request is refused, never a whole tool or command.
    pub fn requires_pro(feature: &str) -> Self {
        let mut e = Self::new(REQUIRES_PRO, format!("{feature} is a Styky Pro feature: {PRO_URL}"));
        e.data = Some(json!({ "feature": feature, "url": PRO_URL }));
        e
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
    let mut error = json!({ "code": e.code, "message": e.message });
    if let Some(d) = &e.data {
        error["data"] = d.clone();
    }
    json!({ "id": id, "error": error }).to_string()
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
    // The pipe name is predictable and machine-wide: before sending anything, make sure the process that created
    // it runs as *us*. (The server also refuses other users through the pipe's ACL; this protects the client.)
    if !server_is_current_user(&pipe) {
        return Err(ApiError::new(
            "untrusted_server",
            "Refusing to talk to the Styky pipe: the process behind it is not running as your Windows user.",
        ));
    }
    let io = |e: std::io::Error| ApiError::new("io", format!("connection to Styky failed: {e}"));
    let mut w = &pipe;
    let req = json!({ "id": 1, "method": method, "params": params });
    w.write_all(format!("{req}\n").as_bytes()).map_err(io)?;
    w.flush().map_err(io)?;
    let mut line = String::new();
    BufReader::new(&pipe).read_line(&mut line).map_err(io)?;
    let v: Value = serde_json::from_str(&line).map_err(|e| ApiError::new("protocol", format!("bad reply from Styky: {e}")))?;
    match v.get("error") {
        Some(e) if !e.is_null() => {
            let mut err = ApiError::new(e["code"].as_str().unwrap_or("error"), e["message"].as_str().unwrap_or("unknown error"));
            err.data = e.get("data").filter(|d| !d.is_null()).cloned();
            Err(err)
        }
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

/// Is the process serving this pipe handle running as the same user as we are? Fails closed.
#[cfg(windows)]
pub fn server_is_current_user(pipe: &std::fs::File) -> bool {
    use std::os::windows::io::AsRawHandle;
    use windows::Win32::Foundation::HANDLE;
    use windows::Win32::System::Pipes::GetNamedPipeServerProcessId;
    let mut pid = 0u32;
    // SAFETY: the handle is a valid open pipe for the lifetime of the call.
    let ok = unsafe { GetNamedPipeServerProcessId(HANDLE(pipe.as_raw_handle()), &mut pid) }.is_ok();
    ok && pid_runs_as_current_user(pid)
}

/// Compare the token user SID of process `pid` with ours. Any failure (no access, gone) is "no".
#[cfg(windows)]
pub fn pid_runs_as_current_user(pid: u32) -> bool {
    use windows::Win32::Foundation::{CloseHandle, HANDLE};
    use windows::Win32::Security::{EqualSid, GetTokenInformation, TOKEN_QUERY, TOKEN_USER, TokenUser};
    use windows::Win32::System::Threading::{GetCurrentProcess, OpenProcess, OpenProcessToken, PROCESS_QUERY_LIMITED_INFORMATION};

    /// TOKEN_USER bytes (the SID lives inside the same buffer).
    unsafe fn token_user(token: HANDLE) -> Option<Vec<u8>> {
        let mut len = 0u32;
        let _ = unsafe { GetTokenInformation(token, TokenUser, None, 0, &mut len) };
        if len == 0 {
            return None;
        }
        let mut buf = vec![0u8; len as usize];
        unsafe { GetTokenInformation(token, TokenUser, Some(buf.as_mut_ptr().cast()), len, &mut len) }.ok()?;
        Some(buf)
    }

    unsafe {
        let mut mine = HANDLE::default();
        if OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut mine).is_err() {
            return false;
        }
        let me = token_user(mine);
        let _ = CloseHandle(mine);
        let Some(me) = me else { return false };

        let Ok(proc) = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid) else { return false };
        let mut theirs = HANDLE::default();
        let opened = OpenProcessToken(proc, TOKEN_QUERY, &mut theirs).is_ok();
        let _ = CloseHandle(proc);
        if !opened {
            return false;
        }
        let other = token_user(theirs);
        let _ = CloseHandle(theirs);
        let Some(other) = other else { return false };
        let (a, b) = (&*(me.as_ptr() as *const TOKEN_USER), &*(other.as_ptr() as *const TOKEN_USER));
        EqualSid(a.User.Sid, b.User.Sid).is_ok()
    }
}

#[cfg(all(test, windows))]
mod tests {
    use super::*;

    #[test]
    fn our_own_process_is_us_and_system_is_not() {
        assert!(pid_runs_as_current_user(std::process::id()));
        // PID 4 is the NT kernel "System" process (SYSTEM account); a normal user cannot even open it, and either way it is refused.
        assert!(!pid_runs_as_current_user(4));
        // A pid that does not exist.
        assert!(!pid_runs_as_current_user(0xFFFF_FFF0));
    }
}
