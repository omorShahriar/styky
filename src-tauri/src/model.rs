//! Data types shared by the DB, commands and export/import.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

pub const PRIORITY_LOW: i64 = 0;
pub const PRIORITY_URGENT: i64 = 2;

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

pub fn normalize_repeat(r: &str) -> &'static str {
    match r {
        "daily" => "daily",
        "weekdays" => "weekdays",
        "weekly" => "weekly",
        "monthly" => "monthly",
        _ => "none",
    }
}

/// Trim and turn empty strings into `None`.
pub fn clean_opt(s: &Option<String>) -> Option<String> {
    s.as_deref()
        .map(str::trim)
        .filter(|v| !v.is_empty())
        .map(str::to_string)
}
