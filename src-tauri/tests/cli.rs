//! End-to-end: the real `styky-cli` binary talking to the real API server over a real named pipe.

#![cfg(windows)]

use std::io::{BufRead, BufReader, Write};
use std::process::{Command, Output, Stdio};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use serde_json::{Value, json};
use styky_lib::{api, db::Db, proto};

const CLI: &str = env!("CARGO_BIN_EXE_styky-cli");

struct Harness {
    user: String,
    pro: Arc<AtomicBool>,
}

/// Start the API on a pipe unique to this test; the CLI is pointed at it through USERNAME.
async fn harness(tag: &str, pro: bool) -> Harness {
    let user = format!("clitest{tag}{}", std::process::id());
    let db_path = std::env::temp_dir().join(format!("styky-{user}.db"));
    let _ = std::fs::remove_file(&db_path);
    let db = Arc::new(Db::open(&db_path).unwrap());
    let flag = Arc::new(AtomicBool::new(pro));
    let f = flag.clone();
    let handler: Arc<dyn Fn(&str) -> String + Send + Sync> = Arc::new(move |line| {
        let ctx = api::Ctx { db: &db, now: chrono::Utc::now().timestamp_millis(), pro: f.load(Ordering::SeqCst) };
        api::handle_line(&ctx, line).0
    });
    let name = format!(r"\\.\pipe\styky-{user}");
    tokio::spawn(async move {
        if let Err(e) = api::serve(name, handler, Arc::new(api::ApiStatus::default())).await {
            eprintln!("serve failed: {e}");
        }
    });
    tokio::time::sleep(std::time::Duration::from_millis(300)).await;
    Harness { user, pro: flag }
}

fn cli(user: &str, args: &[&str]) -> Output {
    Command::new(CLI).args(args).env("USERNAME", user).output().unwrap()
}

fn stdout(o: &Output) -> String {
    String::from_utf8_lossy(&o.stdout).to_string()
}
fn stderr(o: &Output) -> String {
    String::from_utf8_lossy(&o.stderr).to_string()
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn free_tier_add_and_ls_work_and_the_rest_is_gated() {
    let h = harness("free", false).await;
    let u = h.user.clone();
    tokio::task::spawn_blocking(move || {
        let o = cli(&u, &["add", "Release", "v2.3", "tomorrow", "5pm", "#api", "!!"]);
        assert!(o.status.success(), "{}", stderr(&o));
        assert!(stdout(&o).contains("Release v2.3") && stdout(&o).contains("#api"), "{}", stdout(&o));

        let o = cli(&u, &["ls", "--json"]);
        let v: Value = serde_json::from_str(&stdout(&o)).unwrap();
        assert_eq!(v.as_array().unwrap().len(), 1);
        assert_eq!((v[0]["title"].as_str(), v[0]["priority"].as_str()), (Some("Release v2.3"), Some("urgent")));
        let id = v[0]["shortId"].as_str().unwrap().to_string();

        let o = cli(&u, &["ls", "--tag", "nope"]);
        assert_eq!(stdout(&o), "Nothing here.\n");

        for args in [vec!["done", &id], vec!["snooze", &id, "1h"], vec!["rm", &id], vec!["get", &id]] {
            let o = cli(&u, &args);
            assert_eq!(o.status.code(), Some(3), "{args:?}");
            assert!(stderr(&o).contains("requires Styky Pro"), "{}", stderr(&o));
        }
    })
    .await
    .unwrap();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn pro_unlocks_done_snooze_edit_and_rm() {
    let h = harness("pro", true).await;
    let u = h.user.clone();
    tokio::task::spawn_blocking(move || {
        cli(&u, &["add", "write", "docs", "in", "2", "hours"]);
        let v: Value = serde_json::from_str(&stdout(&cli(&u, &["ls", "--json"]))).unwrap();
        let id = v[0]["shortId"].as_str().unwrap().to_string();

        let o = cli(&u, &["snooze", &id, "3h"]);
        assert!(o.status.success(), "{}", stderr(&o));
        let o = cli(&u, &["edit", &id, "--title", "write the docs", "--priority", "low"]);
        assert!(o.status.success(), "{}", stderr(&o));
        let o = cli(&u, &["get", &id]);
        assert!(stdout(&o).contains("write the docs") && stdout(&o).contains("snoozed until"), "{}", stdout(&o));
        let o = cli(&u, &["done", &id]);
        assert!(stdout(&o).starts_with("Done"), "{}", stdout(&o));
        assert_eq!(stdout(&cli(&u, &["ls"])), "Nothing here.\n");
        assert!(stdout(&cli(&u, &["ls", "--done"])).contains("write the docs"));
        assert!(cli(&u, &["rm", &id]).status.success());
        assert_eq!(cli(&u, &["rm", &id]).status.code(), Some(1)); // not found
    })
    .await
    .unwrap();
    drop(h.pro);
}

#[test]
fn a_missing_app_is_reported_clearly() {
    let o = cli("nobody-home-xyz", &["ls"]);
    assert_eq!(o.status.code(), Some(2));
    assert!(stderr(&o).contains("isn't running"), "{}", stderr(&o));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn mcp_server_speaks_stdio_end_to_end() {
    let h = harness("mcp", false).await;
    let u = h.user.clone();
    tokio::task::spawn_blocking(move || {
        let mut child = Command::new(CLI).arg("mcp").env("USERNAME", &u).stdin(Stdio::piped()).stdout(Stdio::piped()).spawn().unwrap();
        let mut stdin = child.stdin.take().unwrap();
        let mut out = BufReader::new(child.stdout.take().unwrap());
        let mut rpc = |msg: Value| -> Value {
            writeln!(stdin, "{msg}").unwrap();
            if msg.get("id").is_none() {
                return Value::Null; // a notification: no reply
            }
            let mut line = String::new();
            out.read_line(&mut line).unwrap();
            serde_json::from_str(&line).unwrap()
        };
        let init = rpc(json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-06-18","capabilities":{},"clientInfo":{"name":"t","version":"0"}}}));
        assert_eq!(init["result"]["serverInfo"]["name"], "styky");
        rpc(json!({"jsonrpc":"2.0","method":"notifications/initialized"}));
        let tools = rpc(json!({"jsonrpc":"2.0","id":2,"method":"tools/list"}));
        assert_eq!(tools["result"]["tools"].as_array().unwrap().len(), 7);
        let add = rpc(json!({"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"styky_add","arguments":{"text":"remind me to rotate the keys friday 10am #infra"}}}));
        assert_eq!(add["result"]["isError"], false, "{add}");
        assert_eq!(add["result"]["structuredContent"]["tag"], "infra");
        let list = rpc(json!({"jsonrpc":"2.0","id":4,"method":"tools/call","params":{"name":"styky_list","arguments":{"tag":"infra"}}}));
        assert_eq!(list["result"]["structuredContent"]["items"].as_array().unwrap().len(), 1);
        let gated = rpc(json!({"jsonrpc":"2.0","id":5,"method":"tools/call","params":{"name":"styky_delete","arguments":{"id":"abcd"}}}));
        assert_eq!(gated["result"]["isError"], true);
        assert!(gated["result"]["content"][0]["text"].as_str().unwrap().contains("requires Styky Pro"));
        drop(stdin);
        let _ = child.wait();
    })
    .await
    .unwrap();
    let _ = proto::pipe_name();
}
