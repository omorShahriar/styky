//! `styky mcp`: a Model Context Protocol server on stdio that exposes the Styky local API as tools.
//!
//! Hand-rolled instead of the official `rmcp` SDK: the server needs only five JSON-RPC methods
//! (`initialize`, `notifications/initialized`, `ping`, `tools/list`, `tools/call`) over newline-delimited
//! stdio, and `rmcp` would add tokio + schemars + proc-macros to the CLI (a much bigger, slower-to-start
//! binary) for no functional gain. Every tool maps 1:1 onto an API method, so schemas are written by hand.

use std::io::{BufRead, Write};

use serde_json::{Value, json};

use crate::CallFn;

/// Protocol revisions this server can speak; the client's request is echoed back if it is one of them.
const SUPPORTED: [&str; 4] = ["2025-11-25", "2025-06-18", "2025-03-26", "2024-11-05"];

const INSTRUCTIONS: &str = "Styky is a Windows desktop reminder widget. Use these tools to add, list and manage the user's reminders. \
Times are interpreted in the user's local time zone. add and list are free; complete, snooze, update, delete and get need Styky Pro \
(if a tool answers 'requires Styky Pro', tell the user instead of retrying). Reminder ids are the 8-character shortId from list/add.";

pub fn tools() -> Value {
    let id = json!({"type": "string", "description": "Reminder id: the 8-character shortId from list/add (any unique prefix of the full id works)."});
    let when = "Natural language (\"tomorrow 5pm\", \"fri 10am\"), a short duration (\"30m\", \"2h\", \"1d\") or an ISO date-time (\"2026-03-12 17:00\", local time, or RFC 3339).";
    let priority = json!({"type": "string", "enum": ["low", "normal", "urgent", "critical"], "description": "critical reminders keep re-notifying until handled (Styky Pro)."});
    json!([
        {
            "name": "styky_add",
            "description": "Add a reminder to Styky. Pass either `text` (a natural-language line such as \"Release v2.3 fri 5pm #api !!\": dates/times, #tag, @branch, URLs, !! urgent, `every day|weekday|week|month` are parsed) or explicit fields. Explicit fields override what the text says. Free.",
            "inputSchema": {"type": "object", "properties": {
                "text": {"type": "string", "description": "Natural-language reminder, e.g. \"rotate the keys friday 10am #infra !!\"."},
                "title": {"type": "string", "description": "Reminder title (overrides the title parsed from text)."},
                "due": {"type": "string", "description": format!("When it is due. {when}")},
                "tag": {"type": "string", "description": "Tag / project, without the #."},
                "priority": priority,
                "repeat": {"type": "string", "enum": ["none", "daily", "weekdays", "weekly", "monthly"]},
                "notes": {"type": "string", "description": "Markdown notes."},
                "link": {"type": "string", "description": "A URL to attach."},
                "branch": {"type": "string", "description": "A git branch / repo to attach."}
            }, "anyOf": [{"required": ["text"]}, {"required": ["title"]}]},
            "annotations": {"title": "Add a reminder", "readOnlyHint": false, "destructiveHint": false, "idempotentHint": false, "openWorldHint": false}
        },
        {
            "name": "styky_list",
            "description": "List the user's reminders, soonest first. Filter by due date, tag and status. Returns id, shortId, title, due (ISO 8601 local), priority, tag, status and whether it is overdue. Free.",
            "inputSchema": {"type": "object", "properties": {
                "due": {"type": "string", "enum": ["today", "overdue", "any"], "description": "today = due today; overdue = past its due/snooze time and still open."},
                "tag": {"type": "string", "description": "Only reminders with this tag."},
                "status": {"type": "string", "enum": ["open", "done", "all"], "description": "Defaults to open."},
                "limit": {"type": "integer", "minimum": 1, "maximum": 1000}
            }},
            "annotations": {"title": "List reminders", "readOnlyHint": true, "openWorldHint": false}
        },
        {
            "name": "styky_get",
            "description": "Get one reminder with all its fields, including notes. Requires Styky Pro.",
            "inputSchema": {"type": "object", "properties": {"id": id}, "required": ["id"]},
            "annotations": {"title": "Get a reminder", "readOnlyHint": true, "openWorldHint": false}
        },
        {
            "name": "styky_complete",
            "description": "Mark a reminder done. A repeating reminder rolls forward to its next occurrence instead. Requires Styky Pro.",
            "inputSchema": {"type": "object", "properties": {"id": id}, "required": ["id"]},
            "annotations": {"title": "Complete a reminder", "readOnlyHint": false, "destructiveHint": false, "idempotentHint": false, "openWorldHint": false}
        },
        {
            "name": "styky_snooze",
            "description": "Snooze a reminder: it will notify again at the given time. Requires Styky Pro.",
            "inputSchema": {"type": "object", "properties": {"id": id, "until": {"type": "string", "description": format!("When to remind again. {when}")}}, "required": ["id", "until"]},
            "annotations": {"title": "Snooze a reminder", "readOnlyHint": false, "destructiveHint": false, "idempotentHint": true, "openWorldHint": false}
        },
        {
            "name": "styky_update",
            "description": "Change fields of a reminder. Only the fields you pass change; pass due: null (or \"\") to clear the due time and tag: null to clear the tag. Requires Styky Pro.",
            "inputSchema": {"type": "object", "properties": {
                "id": id,
                "title": {"type": "string"},
                "due": {"type": ["string", "null"], "description": format!("New due time, or null to clear. {when}")},
                "tag": {"type": ["string", "null"]},
                "priority": priority,
                "repeat": {"type": "string", "enum": ["none", "daily", "weekdays", "weekly", "monthly"]},
                "notes": {"type": "string"},
                "link": {"type": ["string", "null"]},
                "branch": {"type": ["string", "null"]}
            }, "required": ["id"]},
            "annotations": {"title": "Update a reminder", "readOnlyHint": false, "destructiveHint": false, "idempotentHint": true, "openWorldHint": false}
        },
        {
            "name": "styky_delete",
            "description": "Permanently delete a reminder. Prefer styky_complete unless the user asked to delete. Requires Styky Pro.",
            "inputSchema": {"type": "object", "properties": {"id": id}, "required": ["id"]},
            "annotations": {"title": "Delete a reminder", "readOnlyHint": false, "destructiveHint": true, "idempotentHint": true, "openWorldHint": false}
        }
    ])
}

/// Which API method a tool name maps to.
fn method_of(tool: &str) -> Option<&'static str> {
    Some(match tool {
        "styky_add" => "add",
        "styky_list" => "list",
        "styky_get" => "get",
        "styky_complete" => "complete",
        "styky_snooze" => "snooze",
        "styky_update" => "update",
        "styky_delete" => "delete",
        _ => return None,
    })
}

/// Answer one JSON-RPC message. `None` for notifications (and responses we ignore).
pub fn handle(msg: &Value, call: CallFn) -> Option<Value> {
    let method = msg.get("method").and_then(Value::as_str)?;
    let id = msg.get("id").cloned();
    let reply = |result: Result<Value, (i64, String)>| -> Option<Value> {
        let id = id.clone()?; // notifications get no reply
        Some(match result {
            Ok(r) => json!({"jsonrpc": "2.0", "id": id, "result": r}),
            Err((code, m)) => json!({"jsonrpc": "2.0", "id": id, "error": {"code": code, "message": m}}),
        })
    };
    match method {
        "initialize" => {
            let want = msg["params"]["protocolVersion"].as_str().unwrap_or("");
            let version = SUPPORTED.iter().find(|v| **v == want).unwrap_or(&SUPPORTED[0]);
            reply(Ok(json!({
                "protocolVersion": version,
                "capabilities": {"tools": {"listChanged": false}},
                "serverInfo": {"name": "styky", "title": "Styky reminders", "version": env!("CARGO_PKG_VERSION")},
                "instructions": INSTRUCTIONS
            })))
        }
        "ping" => reply(Ok(json!({}))),
        "tools/list" => reply(Ok(json!({"tools": tools()}))),
        "tools/call" => {
            let name = msg["params"]["name"].as_str().unwrap_or("");
            let Some(api) = method_of(name) else {
                return reply(Err((-32602, format!("unknown tool '{name}'"))));
            };
            let args = msg["params"].get("arguments").cloned().unwrap_or_else(|| json!({}));
            reply(Ok(match call(api, args) {
                Ok(v) => json!({
                    "content": [{"type": "text", "text": serde_json::to_string_pretty(&v).unwrap_or_default()}],
                    "structuredContent": if v.is_object() { v } else { json!({"result": v}) },
                    "isError": false
                }),
                // API errors (including "requires Styky Pro" and "Styky isn't running") are tool errors the model can read.
                Err(e) => json!({"content": [{"type": "text", "text": e.message}], "isError": true}),
            }))
        }
        m if m.starts_with("notifications/") => None,
        other => reply(Err((-32601, format!("method not found: {other}")))),
    }
}

/// Serve MCP on stdin/stdout until stdin closes.
pub fn run(call: CallFn) -> i32 {
    let stdin = std::io::stdin();
    let stdout = std::io::stdout();
    for line in stdin.lock().lines() {
        let Ok(line) = line else { break };
        if line.trim().is_empty() {
            continue;
        }
        let out = match serde_json::from_str::<Value>(&line) {
            Ok(msg) => handle(&msg, call),
            Err(e) => Some(json!({"jsonrpc": "2.0", "id": null, "error": {"code": -32700, "message": format!("parse error: {e}")}})),
        };
        if let Some(out) = out {
            let mut w = stdout.lock();
            if writeln!(w, "{out}").and_then(|_| w.flush()).is_err() {
                break;
            }
        }
    }
    0
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::proto::ApiError;

    fn ask(msg: Value, reply: impl Fn(&str, Value) -> Result<Value, ApiError>) -> Option<Value> {
        handle(&msg, &reply)
    }

    #[test]
    fn initialize_negotiates_the_protocol_version() {
        let r = ask(json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-06-18"}}), |_, _| unreachable!()).unwrap();
        assert_eq!(r["result"]["protocolVersion"], "2025-06-18");
        assert_eq!(r["result"]["serverInfo"]["name"], "styky");
        assert!(r["result"]["capabilities"]["tools"].is_object());
        // An unknown future version gets our newest.
        let r = ask(json!({"jsonrpc":"2.0","id":2,"method":"initialize","params":{"protocolVersion":"2099-01-01"}}), |_, _| unreachable!()).unwrap();
        assert_eq!(r["result"]["protocolVersion"], SUPPORTED[0]);
    }

    #[test]
    fn notifications_get_no_reply_and_ping_answers() {
        assert!(ask(json!({"jsonrpc":"2.0","method":"notifications/initialized"}), |_, _| unreachable!()).is_none());
        assert_eq!(ask(json!({"jsonrpc":"2.0","id":3,"method":"ping"}), |_, _| unreachable!()).unwrap()["result"], json!({}));
        assert_eq!(ask(json!({"jsonrpc":"2.0","id":4,"method":"nope"}), |_, _| unreachable!()).unwrap()["error"]["code"], -32601);
    }

    #[test]
    fn tools_list_has_all_seven_with_schemas_and_descriptions() {
        let r = ask(json!({"jsonrpc":"2.0","id":1,"method":"tools/list"}), |_, _| unreachable!()).unwrap();
        let tools = r["result"]["tools"].as_array().unwrap();
        let names: Vec<&str> = tools.iter().map(|t| t["name"].as_str().unwrap()).collect();
        assert_eq!(names, ["styky_add", "styky_list", "styky_get", "styky_complete", "styky_snooze", "styky_update", "styky_delete"]);
        for t in tools {
            assert_eq!(t["inputSchema"]["type"], "object", "{t}");
            assert!(t["description"].as_str().unwrap().len() > 30);
            assert!(method_of(t["name"].as_str().unwrap()).is_some());
        }
        // Pro tools say so, free ones say free.
        for pro in ["styky_get", "styky_complete", "styky_snooze", "styky_update", "styky_delete"] {
            let d = tools.iter().find(|t| t["name"] == pro).unwrap()["description"].as_str().unwrap();
            assert!(d.contains("Requires Styky Pro"), "{pro}");
        }
    }

    #[test]
    fn tools_call_maps_to_the_api_and_returns_text_plus_structured_content() {
        let r = ask(
            json!({"jsonrpc":"2.0","id":9,"method":"tools/call","params":{"name":"styky_add","arguments":{"text":"x"}}}),
            |m, p| {
                assert_eq!((m, p["text"].as_str()), ("add", Some("x")));
                Ok(json!({"shortId": "a1b2c3d4", "title": "x"}))
            },
        )
        .unwrap();
        assert_eq!(r["id"], 9);
        assert_eq!(r["result"]["isError"], false);
        assert!(r["result"]["content"][0]["text"].as_str().unwrap().contains("a1b2c3d4"));
        assert_eq!(r["result"]["structuredContent"]["title"], "x");
    }

    #[test]
    fn api_errors_become_tool_errors_the_model_can_read() {
        let r = ask(
            json!({"jsonrpc":"2.0","id":9,"method":"tools/call","params":{"name":"styky_complete","arguments":{"id":"a1"}}}),
            |_, _| Err(ApiError::new("requires_pro", "'complete' requires Styky Pro.")),
        )
        .unwrap();
        assert_eq!(r["result"]["isError"], true);
        assert!(r["result"]["content"][0]["text"].as_str().unwrap().contains("requires Styky Pro"));
        let r = ask(json!({"jsonrpc":"2.0","id":9,"method":"tools/call","params":{"name":"zzz"}}), |_, _| unreachable!()).unwrap();
        assert_eq!(r["error"]["code"], -32602);
    }
}
