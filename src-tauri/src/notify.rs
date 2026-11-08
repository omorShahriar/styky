//! Windows toasts, built directly on WinRT.
//!
//! `tauri-winrt-notification` (and the Tauri notification plugin) only offer
//! buttons: no `<input type="selection">` and no access to the selected value on
//! activation, which the Snooze dropdown needs. So the toast XML is built here
//! and shown through the `windows` crate. Activation (body click, Done, Snooze)
//! is delivered in-process while Styky is running.

use chrono::{DateTime, Datelike, Local, TimeZone};
use serde_json::json;
use tauri::{AppHandle, Emitter, Manager};
use windows::Data::Xml::Dom::XmlDocument;
use windows::Foundation::{IReference, TypedEventHandler};
use windows::UI::Notifications::{
    ToastActivatedEventArgs, ToastNotification, ToastNotificationManager,
};
use windows::core::{HSTRING, IInspectable, Interface};

use crate::commands::now_ms;
use crate::db::Db;
use crate::model::Item;
use crate::{snooze, window};

/// Same AUMID Windows PowerShell uses; the only one that works for an
/// unregistered (dev) app. Toasts then show "Windows PowerShell" as the source.
const POWERSHELL_AUMID: &str = "{1AC14E77-02E7-4E5D-B744-2EB1AE5198B7}\\WindowsPowerShell\\v1.0\\powershell.exe";

/// What a toast interaction asks for.
#[derive(Debug, PartialEq)]
pub enum Action {
    /// Show the panel (optionally with the custom snooze picker) at an item.
    Open { id: Option<String>, snooze: bool },
    Done(String),
    Snooze { id: String, preset: String },
}

/// `args` is the `launch`/`arguments` string of the toast, `selected` the value of the snooze dropdown.
pub fn parse_action(args: &str, selected: Option<&str>) -> Action {
    let (verb, id) = args.split_once(':').unwrap_or((args, ""));
    let id = (!id.is_empty()).then(|| id.to_string());
    match (verb, id) {
        ("done", Some(id)) => Action::Done(id),
        ("snooze", Some(id)) => match selected.unwrap_or("10m") {
            "custom" => Action::Open { id: Some(id), snooze: true },
            preset => Action::Snooze { id, preset: preset.to_string() },
        },
        ("open", id) => Action::Open { id, snooze: false },
        _ => Action::Open { id: None, snooze: false },
    }
}

fn aumid(app: &AppHandle) -> String {
    if tauri::is_dev() {
        POWERSHELL_AUMID.to_string()
    } else {
        // The NSIS installer stamps its shortcuts with the bundle identifier.
        app.config().identifier.clone()
    }
}

fn esc(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;").replace('"', "&quot;").replace('\'', "&apos;")
}

fn local(ms: i64) -> Option<DateTime<Local>> {
    Local.timestamp_millis_opt(ms).single()
}

/// "Today 17:00", "Tomorrow 09:00", "Mar 5, 09:00".
pub fn when_text(at: i64, now: i64) -> String {
    let (Some(a), Some(n)) = (local(at), local(now)) else { return String::new() };
    let days = a.date_naive().signed_duration_since(n.date_naive()).num_days();
    let time = a.format("%H:%M");
    match days {
        0 => format!("Today {time}"),
        1 => format!("Tomorrow {time}"),
        -1 => format!("Yesterday {time}"),
        _ if a.year() == n.year() => format!("{} {}, {time}", a.format("%b"), a.day()),
        _ => format!("{} {}, {}, {time}", a.format("%b"), a.day(), a.year()),
    }
}

fn item_lines(item: &Item, now: i64) -> (String, String) {
    let mut when = match (item.due_at, item.snoozed_until) {
        (Some(due), Some(_)) => format!("Snoozed · was due {}", when_text(due, now)),
        (Some(due), None) => format!("Due {}", when_text(due, now)),
        (None, _) => "Reminder".to_string(),
    };
    if item.priority == 2 {
        when.push_str(" · Urgent");
    }
    let tag = item.tag.as_ref().map(|t| format!("#{t}")).unwrap_or_default();
    (when, tag)
}

pub fn item_xml(item: &Item, now: i64) -> String {
    let (line1, line2) = item_lines(item, now);
    let id = esc(&item.id);
    let line2 = if line2.is_empty() { String::new() } else { format!("<text>{}</text>", esc(&line2)) };
    format!(
        r#"<toast scenario="reminder" launch="open:{id}" activationType="foreground">
  <visual><binding template="ToastGeneric">
    <text>{title}</text><text>{line1}</text>{line2}
  </binding></visual>
  <actions>
    <input id="snooze" type="selection" defaultInput="10m">
      <selection id="10m" content="10 minutes"/>
      <selection id="1h" content="1 hour"/>
      <selection id="evening" content="This evening (18:00)"/>
      <selection id="tomorrow" content="Tomorrow (09:00)"/>
      <selection id="custom" content="Custom…"/>
    </input>
    <action content="Snooze" arguments="snooze:{id}" activationType="foreground"/>
    <action content="Done" arguments="done:{id}" activationType="foreground"/>
  </actions>
</toast>"#,
        title = esc(&item.title),
        line1 = esc(&line1),
    )
}

pub fn summary_xml(items: &[Item]) -> String {
    let names: Vec<String> = items.iter().take(3).map(|i| esc(&i.title)).collect();
    let more = if items.len() > 3 { format!(" and {} more", items.len() - 3) } else { String::new() };
    format!(
        r#"<toast launch="open:" activationType="foreground">
  <visual><binding template="ToastGeneric">
    <text>{n} reminders were due</text><text>{names}{more}</text>
  </binding></visual>
</toast>"#,
        n = items.len(),
        names = names.join(" · "),
    )
}

fn show_xml(app: &AppHandle, xml: &str, tag: Option<&str>) -> windows::core::Result<()> {
    let doc = XmlDocument::new()?;
    doc.LoadXml(&HSTRING::from(xml))?;
    let toast = ToastNotification::CreateToastNotification(&doc)?;
    if let Some(tag) = tag {
        toast.SetTag(&HSTRING::from(tag))?;
    }
    let handle = app.clone();
    toast.Activated(&TypedEventHandler::<ToastNotification, IInspectable>::new(move |_, insp| {
        if let Some(insp) = insp.as_ref() {
            let args = insp.cast::<ToastActivatedEventArgs>()?;
            let arguments = args.Arguments()?.to_string();
            let selected = args
                .UserInput()
                .and_then(|v| v.Lookup(&HSTRING::from("snooze")))
                .and_then(|v| v.cast::<IReference<HSTRING>>())
                .and_then(|r| r.Value())
                .map(|s| s.to_string())
                .ok();
            handle_action(&handle, parse_action(&arguments, selected.as_deref()));
        }
        Ok(())
    }))?;
    let notifier = ToastNotificationManager::CreateToastNotifierWithId(&HSTRING::from(aumid(app)))?;
    notifier.Show(&toast)
}

/// Toast for one due item.
pub fn show_item(app: &AppHandle, item: &Item) {
    if let Err(e) = show_xml(app, &item_xml(item, now_ms()), Some(&item.id)) {
        eprintln!("styky: toast failed: {e}");
    }
}

/// One toast for several reminders that were missed while the app was off / asleep / paused.
pub fn show_summary(app: &AppHandle, items: &[Item]) {
    if let Err(e) = show_xml(app, &summary_xml(items), None) {
        eprintln!("styky: summary toast failed: {e}");
    }
}

fn handle_action(app: &AppHandle, action: Action) {
    let db = app.state::<Db>();
    let now = now_ms();
    let result = match action {
        Action::Done(id) => db.complete(&id, now).map(|_| ()),
        Action::Snooze { id, preset } => match snooze::preset_until(&preset, now) {
            Some(until) => db.snooze(&id, until, now).map(|_| ()),
            None => Err(format!("unknown snooze preset {preset}")),
        },
        Action::Open { id, snooze } => {
            window::show_panel(app);
            let _ = app.emit("focus-item", json!({ "id": id, "snooze": snooze }));
            return;
        }
    };
    match result {
        Ok(()) => {
            let _ = app.emit("items-changed", ());
        }
        Err(e) => eprintln!("styky: toast action failed: {e}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_toast_arguments() {
        assert_eq!(parse_action("done:abc", None), Action::Done("abc".into()));
        assert_eq!(parse_action("open:abc", None), Action::Open { id: Some("abc".into()), snooze: false });
        assert_eq!(parse_action("open:", None), Action::Open { id: None, snooze: false });
        assert_eq!(
            parse_action("snooze:abc", Some("1h")),
            Action::Snooze { id: "abc".into(), preset: "1h".into() }
        );
        assert_eq!(
            parse_action("snooze:abc", None),
            Action::Snooze { id: "abc".into(), preset: "10m".into() }
        );
        assert_eq!(parse_action("snooze:abc", Some("custom")), Action::Open { id: Some("abc".into()), snooze: true });
        assert_eq!(parse_action("", None), Action::Open { id: None, snooze: false });
    }

    #[test]
    fn xml_is_escaped_and_well_formed_enough() {
        let mut item = Item::blank("id-1");
        item.title = "Fix <b>&\"x\"".into();
        item.tag = Some("api".into());
        item.due_at = Some(1_700_000_000_000);
        let xml = item_xml(&item, 1_700_000_000_000);
        assert!(xml.contains("Fix &lt;b&gt;&amp;&quot;x&quot;"));
        assert!(xml.contains("#api"));
        assert!(xml.contains(&format!("snooze:{}", item.id)));
        assert!(xml.contains("Done"));
    }

    #[test]
    fn when_text_names_nearby_days() {
        let at = |d| Local.with_ymd_and_hms(2026, 3, d, 17, 5, 0).unwrap().timestamp_millis();
        assert_eq!(when_text(at(10), at(10)), "Today 17:05");
        assert_eq!(when_text(at(11), at(10)), "Tomorrow 17:05");
        assert_eq!(when_text(at(9), at(10)), "Yesterday 17:05");
        assert_eq!(when_text(at(20), at(10)), "Mar 20, 17:05");
    }
}
