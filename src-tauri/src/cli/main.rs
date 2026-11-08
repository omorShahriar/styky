//! `styky` command line client for the running Styky app (installed as `styky.exe`, built as `styky-cli`).
//!
//!   styky add "Release v2.3 fri 5pm #api !!"      add a reminder (natural language, same grammar as quick-add)
//!   styky ls [--today|--overdue] [--tag x] [--done|--all] [--json]
//!   styky get <id>                                (Pro)
//!   styky done <id>                               (Pro)
//!   styky snooze <id> 1h|30m|"tomorrow 9am"       (Pro)
//!   styky edit <id> [--title t] [--due when|none] [--tag t] [--priority p] [--notes n]   (Pro)
//!   styky rm <id>                                 (Pro)
//!   styky mcp                                     run an MCP server on stdio for Claude / ChatGPT / any MCP client
//!
//! `add` and `ls` are free; the rest need Styky Pro (the app answers with a clear error).

#[path = "../proto.rs"]
mod proto;

mod mcp;

use chrono::{DateTime, Datelike, FixedOffset, Local};
use proto::ApiError;
use serde_json::{Value, json};

pub type CallFn<'a> = &'a dyn Fn(&str, Value) -> Result<Value, ApiError>;

const HELP: &str = "styky - control the running Styky app from the command line

USAGE
  styky add <text...> [--json]      add a reminder: styky add \"Release v2.3 fri 5pm #api !!\"
  styky ls [--today | --overdue] [--tag <t>] [--done | --all] [--json]
  styky get <id> [--json]           (Styky Pro)
  styky done <id>                   (Styky Pro)
  styky snooze <id> <when>          (Styky Pro)  e.g. 1h, 30m, \"tomorrow 9am\"
  styky edit <id> [--title t] [--due when|none] [--tag t] [--priority low|normal|urgent|critical] [--notes n] [--repeat r]   (Styky Pro)
  styky rm <id>                     (Styky Pro)
  styky mcp                         run an MCP server on stdio (for Claude, ChatGPT, any MCP client)

<id> is the 8-character id shown by `styky ls` (any unique prefix works). `add` and `ls` are free.";

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.first().map(String::as_str) == Some("mcp") {
        std::process::exit(mcp::run(&|m, p| proto::call(m, p)));
    }
    let mut out = String::new();
    let code = run(&args, &|m, p| proto::call(m, p), &mut out);
    if code == 0 {
        print!("{out}");
    } else {
        eprint!("{out}");
    }
    std::process::exit(code);
}

/// Run one command. Output (stdout on success, stderr on failure) is appended to `out`; returns the exit code.
pub fn run(args: &[String], call: CallFn, out: &mut String) -> i32 {
    match exec(args, call, out) {
        Ok(()) => 0,
        Err(e) => {
            out.clear();
            out.push_str(&format!("styky: {}\n", e.message));
            if e.code == "invalid_usage" {
                out.push_str("Try `styky --help`.\n");
            }
            match e.code.as_str() {
                "requires_pro" => 3,
                "not_running" => 2,
                _ => 1,
            }
        }
    }
}

fn usage(msg: impl Into<String>) -> ApiError {
    ApiError::new("invalid_usage", msg)
}

/// Pull `--flag value` out of `args`.
fn flag_value(args: &mut Vec<String>, name: &str) -> Result<Option<String>, ApiError> {
    if let Some(i) = args.iter().position(|a| a == name) {
        if i + 1 >= args.len() {
            return Err(usage(format!("{name} needs a value")));
        }
        let v = args.remove(i + 1);
        args.remove(i);
        return Ok(Some(v));
    }
    Ok(None)
}

fn flag(args: &mut Vec<String>, name: &str) -> bool {
    match args.iter().position(|a| a == name) {
        Some(i) => {
            args.remove(i);
            true
        }
        None => false,
    }
}

fn exec(args: &[String], call: CallFn, out: &mut String) -> Result<(), ApiError> {
    let Some((cmd, rest)) = args.split_first() else {
        out.push_str(HELP);
        out.push('\n');
        return Ok(());
    };
    let mut rest: Vec<String> = rest.to_vec();
    let json_out = flag(&mut rest, "--json");
    let now = Local::now().fixed_offset();
    let show = |v: &Value, out: &mut String| {
        if json_out {
            out.push_str(&serde_json::to_string_pretty(v).unwrap_or_default());
            out.push('\n');
        } else {
            out.push_str(&line(v, now));
            out.push('\n');
        }
    };
    match cmd.as_str() {
        "-h" | "--help" | "help" => {
            out.push_str(HELP);
            out.push('\n');
        }
        "-V" | "--version" | "version" => out.push_str(&format!("styky {}\n", env!("CARGO_PKG_VERSION"))),
        "add" | "a" => {
            let text = rest.join(" ");
            if text.trim().is_empty() {
                return Err(usage("add needs some text, e.g. styky add \"call mom tomorrow 5pm\""));
            }
            let r = call("add", json!({ "text": text }))?;
            if !json_out {
                out.push_str("Added  ");
            }
            show(&r, out);
        }
        "ls" | "list" => {
            let today = flag(&mut rest, "--today");
            let overdue = flag(&mut rest, "--overdue");
            let done = flag(&mut rest, "--done");
            let all = flag(&mut rest, "--all");
            let tag = flag_value(&mut rest, "--tag")?;
            if let Some(extra) = rest.first() {
                return Err(usage(format!("unexpected argument '{extra}'")));
            }
            let mut p = json!({ "status": if all { "all" } else if done { "done" } else { "open" } });
            if today {
                p["due"] = json!("today");
            } else if overdue {
                p["due"] = json!("overdue");
            }
            if let Some(t) = tag {
                p["tag"] = json!(t);
            }
            let r = call("list", p)?;
            let items = r["items"].as_array().cloned().unwrap_or_default();
            if json_out {
                out.push_str(&serde_json::to_string_pretty(&items).unwrap_or_default());
                out.push('\n');
            } else if items.is_empty() {
                out.push_str("Nothing here.\n");
            } else {
                for i in &items {
                    out.push_str(&line(i, now));
                    out.push('\n');
                }
            }
        }
        "get" | "show" => {
            let id = one_id(&rest)?;
            let r = call("get", json!({ "id": id }))?;
            if json_out {
                show(&r, out);
            } else {
                out.push_str(&detail(&r, now));
            }
        }
        "done" | "complete" => {
            let r = call("complete", json!({ "id": one_id(&rest)? }))?;
            if !json_out {
                out.push_str(if r["status"] == "done" { "Done   " } else { "Next   " });
            }
            show(&r, out);
        }
        "snooze" => {
            if rest.len() < 2 {
                return Err(usage("snooze needs an id and a time, e.g. styky snooze a1b2c3d4 1h"));
            }
            let id = rest[0].clone();
            let r = call("snooze", json!({ "id": id, "until": rest[1..].join(" ") }))?;
            if !json_out {
                out.push_str("Snoozed ");
            }
            show(&r, out);
        }
        "edit" | "update" => {
            if rest.is_empty() {
                return Err(usage("edit needs an id"));
            }
            let id = rest.remove(0);
            let mut p = json!({ "id": id });
            for (flag_name, key) in [
                ("--title", "title"),
                ("--tag", "tag"),
                ("--priority", "priority"),
                ("--notes", "notes"),
                ("--repeat", "repeat"),
                ("--link", "link"),
                ("--branch", "branch"),
            ] {
                if let Some(v) = flag_value(&mut rest, flag_name)? {
                    p[key] = json!(v);
                }
            }
            if let Some(v) = flag_value(&mut rest, "--due")? {
                p["due"] = if matches!(v.as_str(), "none" | "clear" | "") { Value::Null } else { json!(v) };
            }
            if let Some(extra) = rest.first() {
                return Err(usage(format!("unexpected argument '{extra}'")));
            }
            let r = call("update", p)?;
            if !json_out {
                out.push_str("Updated ");
            }
            show(&r, out);
        }
        "rm" | "delete" => {
            let r = call("delete", json!({ "id": one_id(&rest)? }))?;
            if json_out {
                show(&r, out);
            } else {
                out.push_str(&format!("Deleted {}\n", r["deleted"].as_str().unwrap_or("")));
            }
        }
        other => return Err(usage(format!("unknown command '{other}'"))),
    }
    Ok(())
}

fn one_id(rest: &[String]) -> Result<String, ApiError> {
    match rest {
        [id] => Ok(id.clone()),
        [] => Err(usage("this command needs a reminder id (see `styky ls`)")),
        _ => Err(usage("expected exactly one id")),
    }
}

/// `a1b2c3d4  Fri 10:00  !!  Title  #tag`
fn line(v: &Value, now: DateTime<FixedOffset>) -> String {
    let when = v["due"].as_str().map(|d| friendly(d, now)).unwrap_or_default();
    let mark = match v["priority"].as_str() {
        Some("critical") => "!!!",
        Some("urgent") => "!!",
        Some("low") => "low",
        _ => "",
    };
    let mut s = format!(
        "{:<8}  {:<13} {:<3}  {}",
        v["shortId"].as_str().unwrap_or("?"),
        when,
        mark,
        v["title"].as_str().unwrap_or("")
    );
    if let Some(t) = v["tag"].as_str() {
        s.push_str(&format!("  #{t}"));
    }
    if v["status"] == "done" {
        s.push_str("  (done)");
    } else if v["overdue"] == true {
        s.push_str("  (overdue)");
    }
    s.trim_end().to_string()
}

fn detail(v: &Value, now: DateTime<FixedOffset>) -> String {
    let mut s = format!("{}\n", line(v, now));
    for (k, label) in [("repeat", "repeat"), ("link", "link"), ("branch", "branch"), ("snoozedUntil", "snoozed until"), ("notes", "notes")] {
        match &v[k] {
            Value::String(t) if !t.is_empty() && !(k == "repeat" && t == "none") => s.push_str(&format!("  {label}: {t}\n")),
            _ => {}
        }
    }
    s.push_str(&format!("  id: {}\n", v["id"].as_str().unwrap_or("")));
    s
}

fn friendly(rfc: &str, now: DateTime<FixedOffset>) -> String {
    let Ok(d) = DateTime::parse_from_rfc3339(rfc) else { return rfc.to_string() };
    let d = d.with_timezone(now.offset());
    let days = (d.date_naive() - now.date_naive()).num_days();
    match days {
        0 => format!("Today {}", d.format("%H:%M")),
        1 => format!("Tomorrow {}", d.format("%H:%M")),
        2..=6 => d.format("%a %H:%M").to_string(),
        _ if d.year() == now.year() => d.format("%d %b %H:%M").to_string(),
        _ => d.format("%Y-%m-%d %H:%M").to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;

    fn s(a: &[&str]) -> Vec<String> {
        a.iter().map(|x| x.to_string()).collect()
    }

    type Recorded = Vec<(String, Value)>;

    /// Records (method, params) and answers from a closure.
    fn exec_with(args: &[&str], reply: impl Fn(&str, &Value) -> Result<Value, ApiError>) -> (i32, String, Recorded) {
        let calls = RefCell::new(vec![]);
        let mut out = String::new();
        let code = run(
            &s(args),
            &|m, p| {
                calls.borrow_mut().push((m.to_string(), p.clone()));
                reply(m, &p)
            },
            &mut out,
        );
        (code, out, calls.into_inner())
    }

    fn item(id: &str, title: &str) -> Value {
        json!({"id": format!("{id}-0000"), "shortId": id, "title": title, "status": "open", "priority": "urgent", "tag": "api", "due": null, "overdue": false, "repeat": "none"})
    }

    #[test]
    fn add_joins_words_and_prints_the_item() {
        let (code, out, calls) = exec_with(&["add", "call", "mom", "tomorrow", "5pm"], |_, _| Ok(item("a1b2c3d4", "call mom")));
        assert_eq!(code, 0);
        assert_eq!(calls[0], ("add".to_string(), json!({"text": "call mom tomorrow 5pm"})));
        assert!(out.starts_with("Added") && out.contains("a1b2c3d4") && out.contains("call mom") && out.contains("#api") && out.contains("!!"), "{out}");
    }

    #[test]
    fn ls_maps_flags_to_filters() {
        let (_, _, c) = exec_with(&["ls", "--today", "--tag", "infra"], |_, _| Ok(json!({"items": []})));
        assert_eq!(c[0].1, json!({"status": "open", "due": "today", "tag": "infra"}));
        let (_, _, c) = exec_with(&["ls", "--overdue"], |_, _| Ok(json!({"items": []})));
        assert_eq!(c[0].1, json!({"status": "open", "due": "overdue"}));
        let (_, _, c) = exec_with(&["ls", "--all"], |_, _| Ok(json!({"items": []})));
        assert_eq!(c[0].1["status"], "all");
        let (code, out, _) = exec_with(&["ls"], |_, _| Ok(json!({"items": []})));
        assert_eq!((code, out.as_str()), (0, "Nothing here.\n"));
    }

    #[test]
    fn ls_json_prints_a_json_array() {
        let (code, out, _) = exec_with(&["ls", "--json"], |_, _| Ok(json!({"items": [item("a1b2c3d4", "x")]})));
        assert_eq!(code, 0);
        let v: Value = serde_json::from_str(&out).unwrap();
        assert_eq!(v[0]["title"], "x");
    }

    #[test]
    fn done_snooze_edit_rm_call_the_right_methods() {
        let (_, _, c) = exec_with(&["done", "a1b2"], |_, _| Ok(item("a1b2c3d4", "x")));
        assert_eq!(c[0], ("complete".to_string(), json!({"id": "a1b2"})));
        let (_, _, c) = exec_with(&["snooze", "a1b2", "tomorrow", "9am"], |_, _| Ok(item("a1b2c3d4", "x")));
        assert_eq!(c[0], ("snooze".to_string(), json!({"id": "a1b2", "until": "tomorrow 9am"})));
        let (_, _, c) = exec_with(&["snooze", "a1b2", "1h"], |_, _| Ok(item("a1b2c3d4", "x")));
        assert_eq!(c[0].1["until"], "1h");
        let (_, _, c) = exec_with(&["edit", "a1b2", "--title", "New", "--due", "none"], |_, _| Ok(item("a1b2c3d4", "x")));
        assert_eq!(c[0], ("update".to_string(), json!({"id": "a1b2", "title": "New", "due": null})));
        let (code, out, c) = exec_with(&["rm", "a1b2"], |_, _| Ok(json!({"deleted": "a1b2c3d4-0000"})));
        assert_eq!((code, c[0].0.as_str()), (0, "delete"));
        assert!(out.contains("Deleted"));
    }

    #[test]
    fn errors_have_exit_codes_and_clear_text() {
        let (code, out, _) = exec_with(&["done", "a1b2"], |_, _| Err(ApiError::new("requires_pro", "'complete' requires Styky Pro.")));
        assert_eq!(code, 3);
        assert!(out.contains("requires Styky Pro"), "{out}");
        let (code, out, _) = exec_with(&["ls"], |_, _| Err(ApiError::new("not_running", proto::NOT_RUNNING)));
        assert_eq!(code, 2);
        assert!(out.contains("isn't running"));
        let (code, _, c) = exec_with(&["add"], |_, _| unreachable!());
        assert_eq!((code, c.len()), (1, 0));
        assert_eq!(exec_with(&["frob"], |_, _| unreachable!()).0, 1);
        assert_eq!(exec_with(&["done"], |_, _| unreachable!()).0, 1);
        assert_eq!(exec_with(&["snooze", "a1"], |_, _| unreachable!()).0, 1);
        assert_eq!(exec_with(&["ls", "--tag"], |_, _| unreachable!()).0, 1);
    }

    #[test]
    fn help_and_version() {
        let (code, out, c) = exec_with(&["--help"], |_, _| unreachable!());
        assert!(code == 0 && out.contains("USAGE") && c.is_empty());
        assert!(exec_with(&["--version"], |_, _| unreachable!()).1.starts_with("styky "));
        assert!(exec_with(&[], |_, _| unreachable!()).1.contains("USAGE"));
    }
}
