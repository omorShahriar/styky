//! Data types shared by the DB, commands and export/import.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

pub const PRIORITY_LOW: i64 = 0;
pub const PRIORITY_URGENT: i64 = 2;
/// Free like every other priority. (Re-notifying an item is the separate, per-item Pro "nag" flag.)
pub const PRIORITY_CRITICAL: i64 = 3;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Item {
    pub id: String,
    pub title: String,
    pub body_md: String,
    pub due_at: Option<i64>,
    pub repeat: String,
    pub priority: i64,
    pub tag: Option<String>,
    pub link: Option<String>,
    pub branch: Option<String>,
    pub status: String,
    pub snoozed_until: Option<i64>,
    pub last_fired_at: Option<i64>,
    pub completed_at: Option<i64>,
    pub sort_order: f64,
    pub created_at: i64,
    pub updated_at: i64,
    /// Day of month a monthly repeat keeps returning to (31st stays 31st through short months).
    #[serde(default)]
    pub repeat_anchor_day: Option<i64>,
    /// Pro: minutes before `due_at` for a heads-up toast (5, 15, 30, 60 or 1440).
    #[serde(default)]
    pub prealert_min: Option<i64>,
    /// When the pre-alert fired for the current due time.
    #[serde(default)]
    pub pre_fired_at: Option<i64>,
    /// Pro: last time this item re-notified (nagging).
    #[serde(default)]
    pub last_nag_at: Option<i64>,
    /// Pro: keep re-notifying every N minutes until the item is done or snoozed. Off by default, on any priority;
    /// never inferred from the priority.
    #[serde(default)]
    pub nag: bool,
    /// Wall-clock time of day (minutes after local midnight) a repeating series keeps returning to, so a
    /// reminder pushed forward by a DST gap goes back to its real time afterwards.
    #[serde(default)]
    pub repeat_anchor_min: Option<i64>,
}

impl Item {
    /// Fire time: `snoozed_until ?? due_at`.
    pub fn effective_at(&self) -> Option<i64> {
        self.snoozed_until.or(self.due_at)
    }

    pub fn is_done(&self) -> bool {
        self.status == "done"
    }
}

/// The user-editable fields (everything the editor can change).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct ItemEdit {
    pub title: String,
    pub body_md: String,
    pub due_at: Option<i64>,
    pub repeat: String,
    pub priority: i64,
    pub tag: Option<String>,
    pub link: Option<String>,
    pub branch: Option<String>,
    pub prealert_min: Option<i64>,
    pub nag: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportFile {
    pub version: u32,
    pub exported_at: i64,
    pub items: Vec<Item>,
    pub settings: BTreeMap<String, String>,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize)]
pub struct ImportResult {
    pub added: usize,
    pub updated: usize,
    pub skipped: usize,
}

/// Legacy simple values pass through; Pro RRULE strings are canonicalised; anything else is "none".
pub fn normalize_repeat(r: &str) -> String {
    crate::repeat::normalize(r)
}

/// Pre-alert lengths the UI offers (minutes).
pub const PREALERTS: [i64; 5] = [5, 15, 30, 60, 1440];

pub fn normalize_prealert(p: Option<i64>) -> Option<i64> {
    p.filter(|m| PREALERTS.contains(m))
}

/// Free builds cannot set Pro-only fields: pre-alerts keep their old value (none for a new item), nagging can be
/// kept or switched off but not switched on, and advanced repeats are not accepted. Every priority, Critical included, is free. Values an item already has are never
/// taken away by an unrelated edit.
pub fn gate_edit(edit: &mut ItemEdit, existing: Option<&Item>, pro: bool) {
    if pro {
        return;
    }
    edit.prealert_min = existing.and_then(|i| i.prealert_min);
    edit.nag = edit.nag && existing.is_some_and(|i| i.nag);
    // Advanced repeats (intervals, chosen weekdays, nth weekday, COUNT/UNTIL) are Pro; the simple ones stay free.
    let wanted = normalize_repeat(&edit.repeat);
    if !crate::repeat::is_simple(&wanted) {
        let keep = existing.map(|i| i.repeat.clone()).filter(|r| *r == wanted);
        edit.repeat = keep.unwrap_or_else(|| existing.map_or_else(|| "none".to_string(), |i| i.repeat.clone()));
    }
}

/// Trim and turn empty strings into `None`.
pub fn clean_opt(s: &Option<String>) -> Option<String> {
    s.as_deref()
        .map(str::trim)
        .filter(|v| !v.is_empty())
        .map(str::to_string)
}

#[cfg(test)]
impl Item {
    /// A plain open item for tests.
    pub fn blank(id: &str) -> Item {
        Item {
            id: id.into(),
            title: id.into(),
            body_md: String::new(),
            due_at: None,
            repeat: "none".into(),
            priority: 1,
            tag: None,
            link: None,
            branch: None,
            status: "open".into(),
            snoozed_until: None,
            last_fired_at: None,
            completed_at: None,
            sort_order: 0.0,
            created_at: 0,
            updated_at: 0,
            repeat_anchor_day: None,
            prealert_min: None,
            pre_fired_at: None,
            last_nag_at: None,
            nag: false,
            repeat_anchor_min: None,
        }
    }
}
