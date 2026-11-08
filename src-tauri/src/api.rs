//! Local API: JSON-RPC style methods over the per-user named pipe (see `proto.rs`).
//!
//! Free: `add`, `list`.  Pro: `get`, `complete`, `snooze`, `update`, `delete`
//! (a free build answers them with the `requires_pro` error). The handlers are plain functions over
//! `Db` so they are unit-tested without a pipe; `serve` is the transport.

use chrono::{DateTime, Local, NaiveDateTime, TimeZone};
use serde_json::{Value, json};

use crate::db::Db;
use crate::model::{Item, ItemEdit, gate_edit};
use crate::nlp;
use crate::proto::{ApiError, err_response, ok_response};

pub struct Ctx<'a> {
    pub db: &'a Db,
    pub now: i64,
    pub pro: bool,
}

type Out = Result<(Value, bool), ApiError>;

/// Why the local API is not serving (shown in Settings → Connect AI); `None` while it is healthy.
#[derive(Default)]
pub struct ApiStatus(std::sync::Mutex<Option<String>>);

impl ApiStatus {
    pub fn set(&self, e: Option<String>) {
        *self.0.lock().unwrap_or_else(|p| p.into_inner()) = e;
    }
    pub fn error(&self) -> Option<String> {
        self.0.lock().unwrap_or_else(|p| p.into_inner()).clone()
    }
}

pub const REQUIRES_PRO: &str = "requires_pro";

fn requires_pro(method: &str) -> ApiError {
    ApiError::new(
        REQUIRES_PRO,
        format!("'{method}' requires Styky Pro. Unlock it in Styky → Settings → Styky Pro. (add and list are always free.)"),
    )
}

fn bad(msg: impl Into<String>) -> ApiError {
    ApiError::new("invalid_params", msg)
}

fn internal(e: String) -> ApiError {
    ApiError::new("internal", e)
}

/// Handle one request line. Returns the response line and whether items changed (caller emits `items-changed`).
pub fn handle_line(ctx: &Ctx, line: &str) -> (String, bool) {
    let req: Value = match serde_json::from_str(line) {
        Ok(v) => v,
        Err(e) => return (err_response(&Value::Null, &ApiError::new("parse", format!("invalid JSON: {e}"))), false),
    };
    let id = req.get("id").cloned().unwrap_or(Value::Null);
    let method = req.get("method").and_then(Value::as_str).unwrap_or("");
    let params = req.get("params").cloned().unwrap_or_else(|| json!({}));
    match dispatch(ctx, method, &params) {
        Ok((v, changed)) => (ok_response(&id, v), changed),
        Err(e) => (err_response(&id, &e), false),
    }
}

fn dispatch(ctx: &Ctx, method: &str, p: &Value) -> Out {
    match method {
        "add" => add(ctx, p),
        "list" => list(ctx, p),
        "get" | "complete" | "snooze" | "update" | "delete" if !ctx.pro => Err(requires_pro(method)),
        "get" => {
            let item = find(ctx, p)?;
            Ok((view(&item, ctx.now), false))
        }
        "complete" => {
            let item = find(ctx, p)?;
            let done = ctx.db.complete(&item.id, ctx.now).map_err(internal)?;
            Ok((view(&done, ctx.now), true))
        }
        "snooze" => snooze(ctx, p),
        "update" => update(ctx, p),
        "delete" => {
            let item = find(ctx, p)?;
            ctx.db.delete(&item.id).map_err(internal)?;
            Ok((json!({ "deleted": item.id }), true))
        }
        "ping" => Ok((json!({ "pro": ctx.pro, "version": env!("CARGO_PKG_VERSION") }), false)),
        "" => Err(ApiError::new("invalid_request", "missing method")),
        m => Err(ApiError::new("unknown_method", format!("unknown method '{m}'"))),
    }
}

// ---------------------------------------------------------------- methods

fn add(ctx: &Ctx, p: &Value) -> Out {
    let now = Local.timestamp_millis_opt(ctx.now).single().ok_or_else(|| internal("bad clock".into()))?;
    let text = p.get("text").and_then(Value::as_str).unwrap_or("");
    let mut q = nlp::parse_quick(text, now);
    // Structured fields (used by MCP clients) override what the sentence said.
    if let Some(t) = str_field(p, "title") {
        q.title = t.to_string();
    }
    if let Some(d) = p.get("due") {
        q.due_at = match d.as_str() {
            Some(s) if !s.trim().is_empty() => Some(parse_when(s, ctx.now).ok_or_else(|| bad(format!("cannot understand due '{s}'")))?),
            _ => None,
        };
    }
    if let Some(v) = str_field(p, "tag") {
        q.tag = Some(v.trim_start_matches('#').to_string());
    }
    if let Some(v) = str_field(p, "link") {
        q.link = Some(v.to_string());
    }
    if let Some(v) = str_field(p, "branch") {
        q.branch = Some(v.to_string());
    }
    if let Some(v) = p.get("priority") {
        q.priority = parse_priority(v)?;
    }
    if let Some(v) = str_field(p, "repeat") {
        q.repeat = parse_repeat(v, ctx.pro)?;
    }
    if q.title.trim().is_empty() {
        return Err(bad("a title is required (pass \"text\" or \"title\")"));
    }
    let prealert = match p.get("prealert") {
        Some(v) => parse_prealert(v)?,
        None => None,
    };
    if !ctx.pro && (prealert.is_some() || q.priority >= 3 && p.get("priority").is_some()) {
        return Err(requires_pro("pre-alerts and Critical priority"));
    }
    let mut edit = ItemEdit {
        title: q.title,
        body_md: str_field(p, "notes").unwrap_or("").to_string(),
        due_at: q.due_at,
        repeat: q.repeat,
        priority: q.priority,
        tag: q.tag,
        link: q.link,
        branch: q.branch,
        prealert_min: prealert,
    };
    gate_edit(&mut edit, None, ctx.pro);
    let item = ctx.db.create(&edit, ctx.now).map_err(internal)?;
    Ok((view(&item, ctx.now), true))
}

fn list(ctx: &Ctx, p: &Value) -> Out {
    let status = str_field(p, "status").unwrap_or("open");
    if !matches!(status, "open" | "done" | "all") {
        return Err(bad("status must be open, done or all"));
    }
    let tag = str_field(p, "tag").map(|t| t.trim_start_matches('#').to_lowercase());
    let due = str_field(p, "due");
    if !matches!(due, None | Some("today") | Some("overdue") | Some("any")) {
        return Err(bad("due must be today, overdue or any"));
    }
    let limit = p.get("limit").and_then(Value::as_u64).unwrap_or(200).min(1000) as usize;
    let (day_start, day_end) = local_day_bounds(ctx.now);
    let items: Vec<Value> = ctx
        .db
        .list(ctx.now)
        .map_err(internal)?
        .into_iter()
        .filter(|i| status == "all" || i.status == status)
        .filter(|i| tag.as_ref().is_none_or(|t| i.tag.as_deref().is_some_and(|x| x.to_lowercase() == *t)))
        .filter(|i| match due {
            Some("today") => !i.is_done() && i.due_at.is_some_and(|d| d >= day_start && d < day_end),
            Some("overdue") => !i.is_done() && i.effective_at().is_some_and(|d| d < ctx.now),
            _ => true,
        })
        .take(limit)
        .map(|i| view(&i, ctx.now))
        .collect();
    Ok((json!({ "items": items }), false))
}

fn snooze(ctx: &Ctx, p: &Value) -> Out {
    let item = find(ctx, p)?;
    let until = str_field(p, "until").ok_or_else(|| bad("until is required, e.g. \"1h\", \"30m\", \"tomorrow 9am\""))?;
    let at = parse_when(until, ctx.now).ok_or_else(|| bad(format!("cannot understand '{until}'")))?;
    if at <= ctx.now {
        return Err(bad("snooze time must be in the future"));
    }
    let s = ctx.db.snooze(&item.id, at, ctx.now).map_err(internal)?;
    Ok((view(&s, ctx.now), true))
}

fn update(ctx: &Ctx, p: &Value) -> Out {
    let item = find(ctx, p)?;
    let mut e = ItemEdit {
        title: item.title.clone(),
        body_md: item.body_md.clone(),
        due_at: item.due_at,
        repeat: item.repeat.clone(),
        priority: item.priority,
        tag: item.tag.clone(),
        link: item.link.clone(),
        branch: item.branch.clone(),
        prealert_min: item.prealert_min,
    };
    if let Some(v) = p.get("prealert") {
        e.prealert_min = parse_prealert(v)?;
    }
    if let Some(v) = str_field(p, "title") {
        e.title = v.to_string();
    }
    if let Some(v) = p.get("notes").and_then(Value::as_str) {
        e.body_md = v.to_string();
    }
    if let Some(d) = p.get("due") {
        e.due_at = match d.as_str() {
            Some(s) if !s.trim().is_empty() => Some(parse_when(s, ctx.now).ok_or_else(|| bad(format!("cannot understand due '{s}'")))?),
            _ => None, // null or "" clears it
        };
    }
    for (key, slot) in [("tag", &mut e.tag), ("link", &mut e.link), ("branch", &mut e.branch)] {
        if let Some(v) = p.get(key) {
            *slot = v.as_str().map(|s| s.trim().trim_start_matches('#').to_string()).filter(|s| !s.is_empty());
        }
    }
    if let Some(v) = p.get("priority") {
        e.priority = parse_priority(v)?;
    }
    if let Some(v) = str_field(p, "repeat") {
        e.repeat = parse_repeat(v, ctx.pro)?;
    }
    if e.title.trim().is_empty() {
        return Err(bad("title cannot be empty"));
    }
    gate_edit(&mut e, Some(&item), ctx.pro);
    let u = ctx.db.update(&item.id, &e, ctx.now).map_err(internal)?;
    Ok((view(&u, ctx.now), true))
}

// ---------------------------------------------------------------- helpers

fn str_field<'a>(p: &'a Value, k: &str) -> Option<&'a str> {
    p.get(k).and_then(Value::as_str).map(str::trim).filter(|s| !s.is_empty())
}

/// Resolve `id` (full uuid or a unique prefix) to an item.
fn find(ctx: &Ctx, p: &Value) -> Result<Item, ApiError> {
    let want = str_field(p, "id").ok_or_else(|| bad("id is required"))?.to_lowercase();
    let all = ctx.db.list(ctx.now).map_err(internal)?;
    let mut hits: Vec<Item> = all.into_iter().filter(|i| i.id == want || i.id.starts_with(&want)).collect();
    match hits.len() {
        0 => Err(ApiError::new("not_found", format!("no reminder with id '{want}'"))),
        1 => Ok(hits.remove(0)),
        n => match hits.iter().position(|i| i.id == want) {
            Some(k) => Ok(hits.remove(k)),
            None => Err(bad(format!("id '{want}' matches {n} reminders; use more characters"))),
        },
    }
}

pub fn priority_name(p: i64) -> &'static str {
    match p {
        0 => "low",
        2 => "urgent",
        3 => "critical",
        _ => "normal",
    }
}

fn parse_priority(v: &Value) -> Result<i64, ApiError> {
    if let Some(n) = v.as_i64() {
        return if (0..=3).contains(&n) { Ok(n) } else { Err(bad("priority must be low, normal, urgent or critical")) };
    }
    match v.as_str().map(str::to_lowercase).as_deref() {
        Some("low") => Ok(0),
        Some("normal") => Ok(1),
        Some("urgent") | Some("high") => Ok(2),
        Some("critical") => Ok(3),
        _ => Err(bad("priority must be low, normal, urgent or critical")),
    }
}

/// `15`, `"15m"`, `"1h"`, `"1d"`, `"none"`/null: minutes before the due time.
fn parse_prealert(v: &Value) -> Result<Option<i64>, ApiError> {
    let mins = match v {
        Value::Null => return Ok(None),
        Value::Number(n) => n.as_i64(),
        Value::String(s) if matches!(s.trim().to_lowercase().as_str(), "" | "none" | "off") => return Ok(None),
        Value::String(s) => parse_duration_ms(s).map(|ms| ms / 60_000),
        _ => None,
    };
    mins.filter(|m| crate::model::PREALERTS.contains(m))
        .map(Some)
        .ok_or_else(|| bad("prealert must be one of 5m, 15m, 30m, 1h, 1d (or null)"))
}

fn parse_repeat(s: &str, pro: bool) -> Result<String, ApiError> {
    let r = match s.to_lowercase().as_str() {
        "" | "none" => "none".to_string(),
        "daily" | "day" => "daily".into(),
        "weekdays" | "weekday" => "weekdays".into(),
        "weekly" | "week" => "weekly".into(),
        "monthly" | "month" => "monthly".into(),
        _ => {
            let n = crate::repeat::normalize(s);
            if n == "none" {
                return Err(bad(format!(
                    "repeat '{s}' is not none, daily, weekdays, weekly, monthly or an RRULE like FREQ=WEEKLY;INTERVAL=2;BYDAY=MO,WE;COUNT=6"
                )));
            }
            if !pro {
                return Err(requires_pro("advanced repeats"));
            }
            n
        }
    };
    Ok(r)
}

/// RFC 3339, `YYYY-MM-DD[ T]HH:MM` (local), a short duration (`30m`, `2h`, `3d`, `1w`) or natural language.
pub fn parse_when(s: &str, now_ms: i64) -> Option<i64> {
    let s = s.trim();
    if let Ok(d) = DateTime::parse_from_rfc3339(s) {
        return Some(d.timestamp_millis());
    }
    for f in ["%Y-%m-%d %H:%M", "%Y-%m-%dT%H:%M", "%Y-%m-%d %H:%M:%S", "%Y-%m-%dT%H:%M:%S"] {
        if let Ok(n) = NaiveDateTime::parse_from_str(s, f) {
            return crate::repeat::from_naive(n);
        }
    }
    if let Some(ms) = parse_duration_ms(s) {
        return Some(now_ms + ms);
    }
    let now = Local.timestamp_millis_opt(now_ms).single()?;
    let q = nlp::parse_quick(s, now);
    q.due_at
}

/// `10m`, `2h`, `1d`, `1w` (also `90 min`, `2 hours`).
pub fn parse_duration_ms(s: &str) -> Option<i64> {
    let s = s.trim().to_lowercase();
    let split = s.find(|c: char| !c.is_ascii_digit())?;
    let (n, unit) = s.split_at(split);
    let n: i64 = n.parse().ok()?;
    let mult = match unit.trim() {
        "m" | "min" | "mins" | "minute" | "minutes" => 60_000,
        "h" | "hr" | "hrs" | "hour" | "hours" => 3_600_000,
        "d" | "day" | "days" => 86_400_000,
        "w" | "week" | "weeks" => 7 * 86_400_000,
        _ => return None,
    };
    Some(n * mult)
}

fn local_day_bounds(now_ms: i64) -> (i64, i64) {
    use chrono::{Days, NaiveTime};
    let Some(now) = Local.timestamp_millis_opt(now_ms).single() else { return (0, i64::MAX) };
    let d = now.date_naive();
    let start = crate::repeat::from_naive(d.and_time(NaiveTime::MIN)).unwrap_or(now_ms);
    let end = d
        .checked_add_days(Days::new(1))
        .and_then(|n| crate::repeat::from_naive(n.and_time(NaiveTime::MIN)))
        .unwrap_or(i64::MAX);
    (start, end)
}

fn rfc(ms: i64) -> Option<String> {
    Local.timestamp_millis_opt(ms).single().map(|d| d.to_rfc3339())
}

/// The public JSON shape of a reminder (stable; used by the CLI, MCP and `--json`).
pub fn view(i: &Item, now: i64) -> Value {
    json!({
        "id": i.id,
        "shortId": &i.id[..8.min(i.id.len())],
        "title": i.title,
        "notes": i.body_md,
        "status": i.status,
        "due": i.due_at.and_then(rfc),
        "dueAt": i.due_at,
        "snoozedUntil": i.snoozed_until.and_then(rfc),
        "overdue": !i.is_done() && i.effective_at().is_some_and(|d| d < now),
        "repeat": i.repeat,
        "priority": priority_name(i.priority),
        "prealertMin": i.prealert_min,
        "tag": i.tag,
        "link": i.link,
        "branch": i.branch,
        "completedAt": i.completed_at.and_then(rfc),
    })
}

// ---------------------------------------------------------------- transport

#[cfg(windows)]
pub use server::serve;

#[cfg(windows)]
mod server {
    use std::ffi::c_void;
    use std::sync::Arc;

    use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};
    use tokio::net::windows::named_pipe::{NamedPipeServer, ServerOptions};
    use windows::Win32::Foundation::{CloseHandle, HANDLE, HLOCAL, LocalFree};
    use windows::Win32::Security::Authorization::{
        ConvertSidToStringSidW, ConvertStringSecurityDescriptorToSecurityDescriptorW, SDDL_REVISION_1,
    };
    use windows::Win32::Security::{GetTokenInformation, PSECURITY_DESCRIPTOR, SECURITY_ATTRIBUTES, TOKEN_QUERY, TOKEN_USER, TokenUser};
    use windows::Win32::System::Threading::{GetCurrentProcess, OpenProcessToken};
    use windows::core::{PCWSTR, PWSTR};

    /// `handler(line)` answers one request line.
    pub type Handler = Arc<dyn Fn(&str) -> String + Send + Sync>;

    /// SID string (`S-1-5-21-…`) of the user running this process.
    fn current_user_sid() -> Option<String> {
        unsafe {
            let mut token = HANDLE::default();
            OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut token).ok()?;
            let mut len = 0u32;
            let _ = GetTokenInformation(token, TokenUser, None, 0, &mut len);
            let mut buf = vec![0u8; len as usize];
            let r = GetTokenInformation(token, TokenUser, Some(buf.as_mut_ptr() as *mut c_void), len, &mut len);
            let sid = if r.is_ok() {
                let tu = &*(buf.as_ptr() as *const TOKEN_USER);
                let mut p = PWSTR::null();
                ConvertSidToStringSidW(tu.User.Sid, &mut p).ok().map(|_| {
                    let s = p.to_string().unwrap_or_default();
                    let _ = LocalFree(Some(HLOCAL(p.0 as *mut c_void)));
                    s
                })
            } else {
                None
            };
            let _ = CloseHandle(token);
            sid.filter(|s| !s.is_empty())
        }
    }

    /// Owns a security descriptor that only lets the current user (and nobody else, not even other
    /// admins or SYSTEM) open the pipe. Remote clients are refused by the pipe itself.
    struct PipeAcl {
        attrs: SECURITY_ATTRIBUTES,
        sd: PSECURITY_DESCRIPTOR,
    }
    // The descriptor is immutable after creation and only read by the kernel.
    unsafe impl Send for PipeAcl {}
    unsafe impl Sync for PipeAcl {}

    impl PipeAcl {
        fn for_current_user() -> Result<Self, String> {
            let sid = current_user_sid().ok_or("cannot read the current user's SID")?;
            let sddl: Vec<u16> = format!("D:P(A;;GA;;;{sid})").encode_utf16().chain(Some(0)).collect();
            let mut sd = PSECURITY_DESCRIPTOR::default();
            unsafe {
                ConvertStringSecurityDescriptorToSecurityDescriptorW(PCWSTR(sddl.as_ptr()), SDDL_REVISION_1, &mut sd, None)
                    .map_err(|e| e.to_string())?;
            }
            Ok(Self {
                attrs: SECURITY_ATTRIBUTES {
                    nLength: std::mem::size_of::<SECURITY_ATTRIBUTES>() as u32,
                    lpSecurityDescriptor: sd.0,
                    bInheritHandle: false.into(),
                },
                sd,
            })
        }

        /// The descriptor as SDDL text (for tests / diagnostics).
        #[cfg(test)]
        fn sddl(&self) -> String {
            use windows::Win32::Security::Authorization::ConvertSecurityDescriptorToStringSecurityDescriptorW;
            use windows::Win32::Security::DACL_SECURITY_INFORMATION;
            let mut out = PWSTR::null();
            unsafe {
                ConvertSecurityDescriptorToStringSecurityDescriptorW(self.sd, SDDL_REVISION_1, DACL_SECURITY_INFORMATION, &mut out, None).unwrap();
                let s = out.to_string().unwrap();
                let _ = LocalFree(Some(HLOCAL(out.0 as *mut c_void)));
                s
            }
        }

        fn create(&self, name: &str, first: bool) -> std::io::Result<NamedPipeServer> {
            let mut opts = ServerOptions::new();
            opts.first_pipe_instance(first).reject_remote_clients(true);
            // SAFETY: `attrs` points at a valid descriptor that outlives this call.
            unsafe { opts.create_with_security_attributes_raw(name, &self.attrs as *const _ as *mut c_void) }
        }
    }

    impl Drop for PipeAcl {
        fn drop(&mut self) {
            unsafe {
                let _ = LocalFree(Some(HLOCAL(self.sd.0)));
            }
        }
    }

    /// Accept connections on `name` until the process ends. Each connection may send many request lines.
    ///
    /// Never gives up: a failed create or connect (a client that vanished early gives ERROR_NO_DATA, another
    /// process squatting the name, resource errors) is recorded in `status`, retried with a short backoff and
    /// the loop continues, so one bad moment cannot disable the API until the next app start.
    pub async fn serve(name: String, handler: Handler, status: Arc<super::ApiStatus>) -> Result<(), String> {
        let acl = match PipeAcl::for_current_user() {
            Ok(a) => Arc::new(a),
            Err(e) => {
                // Deterministic failure: retrying cannot help.
                status.set(Some(format!("cannot build the pipe's access list: {e}")));
                return Err(e);
            }
        };
        let mut first = true; // `first_pipe_instance`: fail instead of joining a pipe another process created under our name
        let mut next: Option<NamedPipeServer> = None;
        let mut backoff = std::time::Duration::from_millis(100);
        loop {
            let server = match next.take() {
                Some(s) => s,
                None => match acl.create(&name, first) {
                    Ok(s) => {
                        first = false;
                        backoff = std::time::Duration::from_millis(100);
                        status.set(None);
                        s
                    }
                    Err(e) => {
                        let msg = format!("cannot create {name}: {e}");
                        eprintln!("styky: local API: {msg}");
                        status.set(Some(msg));
                        tokio::time::sleep(backoff).await;
                        backoff = (backoff * 2).min(std::time::Duration::from_secs(5));
                        continue;
                    }
                },
            };
            if let Err(e) = server.connect().await {
                // ERROR_NO_DATA (232) / broken pipe: the client went away before we could accept. Normal.
                eprintln!("styky: local API connect: {e}");
                tokio::time::sleep(std::time::Duration::from_millis(50)).await;
                continue;
            }
            // Prepare the next instance before serving this one, so there is always a listener.
            match acl.create(&name, false) {
                Ok(s) => next = Some(s),
                Err(e) => {
                    eprintln!("styky: local API: cannot create the next pipe instance: {e}");
                    status.set(Some(format!("cannot create {name}: {e}")));
                }
            }
            let conn = server;
            let h = handler.clone();
            tokio::spawn(async move {
                let (r, mut w) = tokio::io::split(conn);
                let mut r = BufReader::new(r).take(1 << 20);
                let mut line = String::new();
                loop {
                    line.clear();
                    r.set_limit(1 << 20);
                    match r.read_line(&mut line).await {
                        Ok(0) | Err(_) => break,
                        Ok(_) => {
                            let line2 = line.clone();
                            let h2 = h.clone();
                            // Handlers touch SQLite: keep them off the async workers.
                            let out = tokio::task::spawn_blocking(move || h2(line2.trim()))
                                .await
                                .unwrap_or_else(|_| r#"{"id":null,"error":{"code":"internal","message":"handler crashed"}}"#.into());
                            if w.write_all(format!("{out}\n").as_bytes()).await.is_err() {
                                break;
                            }
                        }
                    }
                }
            });
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        use crate::proto;
        use serde_json::json;

        #[test]
        fn the_acl_grants_only_the_current_user() {
            let acl = PipeAcl::for_current_user().unwrap();
            let sid = current_user_sid().unwrap();
            let sddl = acl.sddl();
            assert!(sddl.starts_with("D:P"), "{sddl}");
            assert_eq!(sddl.matches("(A;").count(), 1, "exactly one ACE: {sddl}");
            assert!(sddl.contains(&sid), "{sddl}");
        }

        #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
        async fn requests_round_trip_over_a_real_pipe() {
            let name = format!(r"\\.\pipe\styky-test-{}", std::process::id());
            let h: Handler = Arc::new(|line| format!(r#"{{"id":1,"result":{{"echo":{}}}}}"#, serde_json::to_string(line).unwrap()));
            let n = name.clone();
            tokio::spawn(async move { serve(n, h, Arc::new(crate::api::ApiStatus::default())).await });
            tokio::time::sleep(std::time::Duration::from_millis(200)).await;
            let n = name.clone();
            let r = tokio::task::spawn_blocking(move || proto::call_on(&n, "ping", json!({"a": 1}))).await.unwrap().unwrap();
            assert!(r["echo"].as_str().unwrap().contains(r#""method":"ping""#));
            // A second connection works too (the next instance was created).
            let r2 = tokio::task::spawn_blocking(move || proto::call_on(&name, "ping", json!({}))).await.unwrap();
            assert!(r2.is_ok());
        }

        #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
        async fn clients_that_vanish_do_not_stop_the_server() {
            let name = format!(r"\\.\pipe\styky-test-vanish-{}", std::process::id());
            let h: Handler = Arc::new(|_| r#"{"id":1,"result":"ok"}"#.to_string());
            let n = name.clone();
            let status = Arc::new(crate::api::ApiStatus::default());
            tokio::spawn(async move { serve(n, h, status).await });
            tokio::time::sleep(std::time::Duration::from_millis(200)).await;
            // Connect and drop at once, many times (each leaves the server with a half-open instance).
            let n = name.clone();
            tokio::task::spawn_blocking(move || {
                for _ in 0..40 {
                    drop(std::fs::OpenOptions::new().read(true).write(true).open(&n));
                }
            })
            .await
            .unwrap();
            let n = name.clone();
            let r = tokio::task::spawn_blocking(move || proto::call_on(&n, "ping", json!({}))).await.unwrap();
            assert_eq!(r.unwrap(), json!("ok"));
        }

        #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
        async fn a_squatted_pipe_name_is_reported_and_recovered_from() {
            let name = format!(r"\\.\pipe\styky-test-squat-{}", std::process::id());
            // Someone else creates the name first.
            let squatter = ServerOptions::new().first_pipe_instance(true).create(&name).unwrap();
            let h: Handler = Arc::new(|_| r#"{"id":1,"result":"ok"}"#.to_string());
            let status = Arc::new(crate::api::ApiStatus::default());
            let (n, st) = (name.clone(), status.clone());
            tokio::spawn(async move { serve(n, h, st).await });
            tokio::time::sleep(std::time::Duration::from_millis(400)).await;
            assert!(status.error().unwrap_or_default().contains("cannot create"), "{:?}", status.error());
            // The squatter goes away: the server takes the name over on its own and the error clears.
            drop(squatter);
            tokio::time::sleep(std::time::Duration::from_millis(1500)).await;
            assert_eq!(status.error(), None);
            let n = name.clone();
            let r = tokio::task::spawn_blocking(move || proto::call_on(&n, "ping", json!({}))).await.unwrap();
            assert_eq!(r.unwrap(), json!("ok"));
        }

        #[test]
        fn a_missing_pipe_means_styky_is_not_running() {
            let e = proto::call_on(r"\\.\pipe\styky-test-does-not-exist", "ping", json!({})).unwrap_err();
            assert_eq!(e.code, "not_running");
        }
    }
}

// ---------------------------------------------------------------- tests

#[cfg(test)]
mod tests {
    use super::*;

    const NOW: i64 = 1_773_223_200_000; // 2026-03-11 10:00 UTC-ish; only relative values matter

    fn run(db: &Db, pro: bool, method: &str, params: Value) -> Result<Value, ApiError> {
        let req = json!({"id": 7, "method": method, "params": params}).to_string();
        let (out, _) = handle_line(&Ctx { db, now: NOW, pro }, &req);
        let v: Value = serde_json::from_str(&out).unwrap();
        assert_eq!(v["id"], 7);
        match v.get("error") {
            Some(e) => Err(ApiError::new(e["code"].as_str().unwrap(), e["message"].as_str().unwrap())),
            None => Ok(v["result"].clone()),
        }
    }

    fn add(db: &Db, text: &str) -> Value {
        run(db, false, "add", json!({ "text": text })).unwrap()
    }

    #[test]
    fn add_uses_the_nl_parser_and_is_free() {
        let db = Db::open_memory().unwrap();
        let r = add(&db, "Release v2.3 #api !!");
        assert_eq!((r["title"].as_str(), r["tag"].as_str(), r["priority"].as_str()), (Some("Release v2.3"), Some("api"), Some("urgent")));
        let r = add(&db, "rotate keys in 2 hours");
        assert_eq!(r["dueAt"], NOW + 2 * 3_600_000);
        assert_eq!(r["overdue"], false);
    }

    #[test]
    fn add_accepts_structured_fields_and_rejects_bad_input() {
        let db = Db::open_memory().unwrap();
        let r = run(&db, false, "add", json!({"title": "Ship", "due": "2030-01-02 15:30", "priority": "urgent", "repeat": "weekly", "notes": "- a"})).unwrap();
        assert_eq!((r["priority"].as_str(), r["repeat"].as_str(), r["notes"].as_str()), (Some("urgent"), Some("weekly"), Some("- a")));
        assert!(r["due"].as_str().unwrap().starts_with("2030-01-02T15:30"));
        assert_eq!(run(&db, false, "add", json!({})).unwrap_err().code, "invalid_params");
        assert_eq!(run(&db, false, "add", json!({"title": "x", "due": "never-ish"})).unwrap_err().code, "invalid_params");
        assert_eq!(run(&db, false, "add", json!({"title": "x", "repeat": "yearly"})).unwrap_err().code, "invalid_params");
    }

    #[test]
    fn critical_and_prealert_are_pro_in_the_api_too() {
        let db = Db::open_memory().unwrap();
        // Explicitly asking a free build for them is an error...
        assert_eq!(run(&db, false, "add", json!({"title": "x", "priority": "critical"})).unwrap_err().code, REQUIRES_PRO);
        assert_eq!(run(&db, false, "add", json!({"title": "x", "due": "1h", "prealert": "15m"})).unwrap_err().code, REQUIRES_PRO);
        // ...while the `!!!` token in free text degrades to Urgent.
        assert_eq!(add(&db, "ship it !!!")["priority"], "urgent");
        // Pro: both work, and update can change them.
        let r = run(&db, true, "add", json!({"title": "x", "due": "2h", "priority": "critical", "prealert": "30m"})).unwrap();
        assert_eq!((r["priority"].as_str(), r["prealertMin"].as_i64()), (Some("critical"), Some(30)));
        let u = run(&db, true, "update", json!({"id": r["id"], "prealert": null})).unwrap();
        assert!(u["prealertMin"].is_null());
        assert!(run(&db, true, "update", json!({"id": r["id"], "prealert": "7m"})).is_err());
    }

    #[test]
    fn list_filters_are_free() {
        let db = Db::open_memory().unwrap();
        add(&db, "a #x");
        add(&db, "b #y in 1 hour");
        let r = run(&db, false, "list", json!({})).unwrap();
        assert_eq!(r["items"].as_array().unwrap().len(), 2);
        let r = run(&db, false, "list", json!({"tag": "#X"})).unwrap();
        assert_eq!(r["items"].as_array().unwrap().len(), 1);
        assert_eq!(r["items"][0]["title"], "a");
        assert_eq!(run(&db, false, "list", json!({"status": "done"})).unwrap()["items"].as_array().unwrap().len(), 0);
        assert!(run(&db, false, "list", json!({"status": "nope"})).is_err());
    }

    #[test]
    fn list_today_and_overdue() {
        let db = Db::open_memory().unwrap();
        // NOW is the context clock; items are created relative to it.
        let past = run(&db, false, "add", json!({"title": "late", "due": "30m"})).unwrap();
        assert!(past["dueAt"].as_i64().unwrap() > NOW);
        db.update(
            past["id"].as_str().unwrap(),
            &ItemEdit { title: "late".into(), repeat: "none".into(), priority: 1, due_at: Some(NOW - 3_600_000), ..Default::default() },
            NOW,
        )
        .unwrap();
        run(&db, false, "add", json!({"title": "someday"})).unwrap();
        let over = run(&db, false, "list", json!({"due": "overdue"})).unwrap();
        assert_eq!(over["items"].as_array().unwrap().len(), 1);
        assert_eq!(over["items"][0]["overdue"], true);
        let any = run(&db, false, "list", json!({"due": "any"})).unwrap();
        assert_eq!(any["items"].as_array().unwrap().len(), 2);
    }

    #[test]
    fn pro_methods_are_gated_with_a_clear_error() {
        let db = Db::open_memory().unwrap();
        let id = add(&db, "task")["id"].as_str().unwrap().to_string();
        for (m, p) in [
            ("get", json!({"id": id})),
            ("complete", json!({"id": id})),
            ("snooze", json!({"id": id, "until": "1h"})),
            ("update", json!({"id": id, "title": "x"})),
            ("delete", json!({"id": id})),
        ] {
            let e = run(&db, false, m, p).unwrap_err();
            assert_eq!(e.code, REQUIRES_PRO, "{m}");
            assert!(e.message.contains("requires Styky Pro") && e.message.contains(m), "{}", e.message);
        }
        // Nothing happened.
        assert_eq!(db.list(NOW).unwrap().len(), 1);
    }

    #[test]
    fn pro_methods_work_with_a_short_id() {
        let db = Db::open_memory().unwrap();
        let r = add(&db, "task #t");
        let id = r["id"].as_str().unwrap();
        let short = &id[..8];
        assert_eq!(run(&db, true, "get", json!({"id": short})).unwrap()["title"], "task");
        let s = run(&db, true, "snooze", json!({"id": short, "until": "1h"})).unwrap();
        assert!(s["snoozedUntil"].as_str().is_some());
        assert_eq!(db.list(NOW).unwrap()[0].snoozed_until, Some(NOW + 3_600_000));
        assert!(run(&db, true, "snooze", json!({"id": short, "until": "garbage"})).is_err());
        let u = run(&db, true, "update", json!({"id": short, "title": "task 2", "tag": null, "priority": "low"})).unwrap();
        assert_eq!((u["title"].as_str(), u["tag"].is_null(), u["priority"].as_str()), (Some("task 2"), true, Some("low")));
        let c = run(&db, true, "complete", json!({"id": short})).unwrap();
        assert_eq!(c["status"], "done");
        assert_eq!(run(&db, true, "list", json!({"status": "done"})).unwrap()["items"].as_array().unwrap().len(), 1);
        run(&db, true, "delete", json!({"id": short})).unwrap();
        assert_eq!(run(&db, true, "get", json!({"id": short})).unwrap_err().code, "not_found");
    }

    #[test]
    fn complete_rolls_a_repeating_item_forward() {
        let db = Db::open_memory().unwrap();
        let r = run(&db, false, "add", json!({"title": "daily", "due": "2030-01-02 09:00", "repeat": "daily"})).unwrap();
        let c = run(&db, true, "complete", json!({"id": r["id"]})).unwrap();
        assert_eq!(c["status"], "open");
        assert!(c["due"].as_str().unwrap().starts_with("2030-01-03T09:00"));
    }

    #[test]
    fn ambiguous_unknown_and_malformed_requests() {
        let db = Db::open_memory().unwrap();
        add(&db, "a");
        add(&db, "b");
        // A one-character prefix matches both uuids at most of the time; "" is rejected outright.
        assert_eq!(run(&db, true, "get", json!({})).unwrap_err().code, "invalid_params");
        assert_eq!(run(&db, true, "get", json!({"id": "zzzz"})).unwrap_err().code, "not_found");
        assert_eq!(run(&db, true, "frobnicate", json!({})).unwrap_err().code, "unknown_method");
        let (out, changed) = handle_line(&Ctx { db: &db, now: NOW, pro: true }, "{nope");
        assert!(out.contains("\"parse\"") && !changed);
    }

    #[test]
    fn durations() {
        assert_eq!(parse_duration_ms("90 min"), Some(5_400_000));
        assert_eq!(parse_duration_ms("2h"), Some(7_200_000));
        assert_eq!(parse_duration_ms("1w"), Some(604_800_000));
        assert_eq!(parse_duration_ms("soon"), None);
        assert_eq!(parse_duration_ms("5"), None);
    }
}
