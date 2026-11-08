//! SQLite storage (rusqlite, bundled). Rust owns data and time.

use std::collections::BTreeMap;
use std::path::Path;
use std::sync::{Mutex, MutexGuard};

use rusqlite::{Connection, OptionalExtension, Row, params};

use crate::model::{
    ExportFile, ImportResult, Item, ItemEdit, PRIORITY_CRITICAL, PRIORITY_LOW, clean_opt,
    normalize_prealert, normalize_repeat,
};
use crate::license::Entitlement;
use crate::{repeat, sort};

pub type Res<T> = Result<T, String>;

fn err<E: std::fmt::Display>(e: E) -> String {
    e.to_string()
}

/// Default settings; stored values override them.
pub const DEFAULT_SETTINGS: [(&str, &str); 6] = [
    ("autoclear_days", "7"),
    ("quickadd_hotkey", "Ctrl+Alt+N"),
    ("theme", "system"),
    ("default_snooze", "10m"),
    // Pro: minutes between Critical re-notifications, and whether a re-notification raises the panel.
    ("escalation_minutes", "10"),
    ("escalation_bring_forward", "0"),
];

const SCHEMA_V1: &str = "
CREATE TABLE items (
  id TEXT PRIMARY KEY,
  title TEXT NOT NULL,
  body_md TEXT NOT NULL DEFAULT '',
  due_at INTEGER,
  repeat TEXT NOT NULL DEFAULT 'none',
  priority INTEGER NOT NULL DEFAULT 1,
  tag TEXT,
  link TEXT,
  branch TEXT,
  status TEXT NOT NULL DEFAULT 'open',
  snoozed_until INTEGER,
  last_fired_at INTEGER,
  completed_at INTEGER,
  sort_order REAL NOT NULL,
  created_at INTEGER NOT NULL,
  updated_at INTEGER NOT NULL
);
CREATE TABLE settings (key TEXT PRIMARY KEY, value TEXT NOT NULL);
";

const SCHEMA_V2: &str = "ALTER TABLE items ADD COLUMN repeat_anchor_day INTEGER;";

/// Pro license cache (kept out of `settings` so exports never carry the key).
const SCHEMA_V3: &str = "CREATE TABLE license (
  id INTEGER PRIMARY KEY CHECK (id = 1),
  key TEXT NOT NULL,
  instance_id TEXT NOT NULL,
  validated_at INTEGER NOT NULL,
  customer TEXT
);";

/// Pre-alerts and Critical escalation (Pro).
const SCHEMA_V4: &str = "ALTER TABLE items ADD COLUMN prealert_min INTEGER;
   ALTER TABLE items ADD COLUMN pre_fired_at INTEGER;
   ALTER TABLE items ADD COLUMN last_nag_at INTEGER;";

const COLS: &str = "id, title, body_md, due_at, repeat, priority, tag, link, branch, status, \
                    snoozed_until, last_fired_at, completed_at, sort_order, created_at, updated_at, repeat_anchor_day, \n                    prealert_min, pre_fired_at, last_nag_at";

fn row_to_item(r: &Row) -> rusqlite::Result<Item> {
    Ok(Item {
        id: r.get(0)?,
        title: r.get(1)?,
        body_md: r.get(2)?,
        due_at: r.get(3)?,
        repeat: r.get(4)?,
        priority: r.get(5)?,
        tag: r.get(6)?,
        link: r.get(7)?,
        branch: r.get(8)?,
        status: r.get(9)?,
        snoozed_until: r.get(10)?,
        last_fired_at: r.get(11)?,
        completed_at: r.get(12)?,
        sort_order: r.get(13)?,
        created_at: r.get(14)?,
        updated_at: r.get(15)?,
        repeat_anchor_day: r.get(16)?,
        prealert_min: r.get(17)?,
        pre_fired_at: r.get(18)?,
        last_nag_at: r.get(19)?,
    })
}

fn upsert(conn: &Connection, i: &Item) -> rusqlite::Result<()> {
    conn.execute(
        &format!(
            "INSERT OR REPLACE INTO items ({COLS}) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16,?17,?18,?19,?20)"
        ),
        params![
            i.id,
            i.title,
            i.body_md,
            i.due_at,
            i.repeat,
            i.priority,
            i.tag,
            i.link,
            i.branch,
            i.status,
            i.snoozed_until,
            i.last_fired_at,
            i.completed_at,
            i.sort_order,
            i.created_at,
            i.updated_at,
            i.repeat_anchor_day,
            i.prealert_min,
            i.pre_fired_at,
            i.last_nag_at
        ],
    )?;
    Ok(())
}

pub struct Db(Mutex<Connection>);

impl Db {
    pub fn open(path: &Path) -> Res<Self> {
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir).map_err(err)?;
        }
        Self::init(Connection::open(path).map_err(err)?)
    }

    #[cfg(test)]
    pub fn open_memory() -> Res<Self> {
        Self::init(Connection::open_in_memory().map_err(err)?)
    }

    fn init(conn: Connection) -> Res<Self> {
        conn.pragma_update(None, "journal_mode", "WAL").map_err(err)?;
        migrate(&conn)?;
        Ok(Self(Mutex::new(conn)))
    }

    fn conn(&self) -> MutexGuard<'_, Connection> {
        self.0.lock().unwrap_or_else(|p| p.into_inner())
    }

    fn get(conn: &Connection, id: &str) -> Res<Item> {
        conn.query_row(
            &format!("SELECT {COLS} FROM items WHERE id = ?1"),
            [id],
            row_to_item,
        )
        .optional()
        .map_err(err)?
        .ok_or_else(|| format!("item {id} not found"))
    }

    pub fn get_item(&self, id: &str) -> Res<Item> {
        Self::get(&self.conn(), id)
    }

    /// All items in display order (open first, then completed).
    pub fn list(&self, now: i64) -> Res<Vec<Item>> {
        let conn = self.conn();
        let mut stmt = conn
            .prepare(&format!("SELECT {COLS} FROM items"))
            .map_err(err)?;
        let mut items = stmt
            .query_map([], row_to_item)
            .map_err(err)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(err)?;
        sort::sort_items(&mut items, now);
        Ok(items)
    }

    /// Open items only (the scheduler's working set).
    pub fn list_open(&self) -> Res<Vec<Item>> {
        let conn = self.conn();
        let mut stmt = conn
            .prepare(&format!("SELECT {COLS} FROM items WHERE status = 'open'"))
            .map_err(err)?;
        stmt.query_map([], row_to_item)
            .map_err(err)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(err)
    }

    /// Record that these items' pre-alert fired.
    pub fn mark_pre_fired(&self, ids: &[String], now: i64) -> Res<()> {
        let conn = self.conn();
        for id in ids {
            conn.execute("UPDATE items SET pre_fired_at = ?1 WHERE id = ?2", params![now, id]).map_err(err)?;
        }
        Ok(())
    }

    /// Record that these Critical items re-notified.
    pub fn mark_nagged(&self, ids: &[String], now: i64) -> Res<()> {
        let conn = self.conn();
        for id in ids {
            conn.execute("UPDATE items SET last_nag_at = ?1 WHERE id = ?2", params![now, id]).map_err(err)?;
        }
        Ok(())
    }

    /// Record that these items have fired (does not touch `updated_at`).
    pub fn mark_fired(&self, ids: &[String], now: i64) -> Res<()> {
        let conn = self.conn();
        for id in ids {
            conn.execute("UPDATE items SET last_fired_at = ?1 WHERE id = ?2", params![now, id])
                .map_err(err)?;
        }
        Ok(())
    }

    pub fn create(&self, edit: &ItemEdit, now: i64) -> Res<Item> {
        let conn = self.conn();
        let max: Option<f64> = conn
            .query_row("SELECT MAX(sort_order) FROM items", [], |r| r.get(0))
            .map_err(err)?;
        let item = Item {
            id: uuid::Uuid::new_v4().to_string(),
            title: edit.title.trim().to_string(),
            body_md: edit.body_md.clone(),
            due_at: edit.due_at,
            repeat: normalize_repeat(&edit.repeat).to_string(),
            priority: edit.priority.clamp(PRIORITY_LOW, PRIORITY_CRITICAL),
            tag: clean_opt(&edit.tag),
            link: clean_opt(&edit.link),
            branch: clean_opt(&edit.branch),
            status: "open".into(),
            snoozed_until: None,
            last_fired_at: None,
            completed_at: None,
            sort_order: max.unwrap_or(0.0) + 1.0,
            created_at: now,
            updated_at: now,
            repeat_anchor_day: None,
            prealert_min: normalize_prealert(edit.prealert_min),
            pre_fired_at: None,
            last_nag_at: None,
        };
        let mut item = item;
        item.repeat_anchor_day = anchor_for(&item.repeat, item.due_at);
        upsert(&conn, &item).map_err(err)?;
        Ok(item)
    }

    pub fn update(&self, id: &str, edit: &ItemEdit, now: i64) -> Res<Item> {
        let conn = self.conn();
        let mut item = Self::get(&conn, id)?;
        let repeat = normalize_repeat(&edit.repeat);
        let re_anchor = item.due_at != edit.due_at || item.repeat != repeat;
        if item.due_at != edit.due_at {
            // A new due time re-arms the reminder.
            item.snoozed_until = None;
            item.last_fired_at = None;
            item.pre_fired_at = None;
            item.last_nag_at = None;
        }
        let prealert = normalize_prealert(edit.prealert_min);
        if item.prealert_min != prealert {
            item.pre_fired_at = None;
            item.prealert_min = prealert;
        }
        item.title = edit.title.trim().to_string();
        item.body_md = edit.body_md.clone();
        item.due_at = edit.due_at;
        item.repeat = normalize_repeat(&edit.repeat).to_string();
        item.priority = edit.priority.clamp(PRIORITY_LOW, PRIORITY_CRITICAL);
        item.tag = clean_opt(&edit.tag);
        item.link = clean_opt(&edit.link);
        item.branch = clean_opt(&edit.branch);
        item.updated_at = now;
        if re_anchor || item.repeat_anchor_day.is_none() {
            item.repeat_anchor_day = anchor_for(&item.repeat, item.due_at);
        }
        upsert(&conn, &item).map_err(err)?;
        Ok(item)
    }

    pub fn delete(&self, id: &str) -> Res<()> {
        self.conn()
            .execute("DELETE FROM items WHERE id = ?1", [id])
            .map_err(err)?;
        Ok(())
    }

    /// Put back a previously deleted item exactly as it was (undo).
    pub fn undelete(&self, item: &Item) -> Res<()> {
        upsert(&self.conn(), item).map_err(err)
    }

    /// Complete an item. Repeating items roll forward instead of being marked done.
    pub fn complete(&self, id: &str, now: i64) -> Res<Item> {
        let conn = self.conn();
        let mut item = Self::get(&conn, id)?;
        let next = item
            .due_at
            .and_then(|due| repeat::next_occurrence(due, &item.repeat, now, item.repeat_anchor_day.map(|d| d as u32)));
        match next {
            Some(next_due) => {
                item.due_at = Some(next_due);
                item.snoozed_until = None;
                item.last_fired_at = None;
                item.pre_fired_at = None;
                item.last_nag_at = None;
            }
            None => {
                item.status = "done".into();
                item.completed_at = Some(now);
            }
        }
        item.updated_at = now;
        upsert(&conn, &item).map_err(err)?;
        Ok(item)
    }

    pub fn restore(&self, id: &str, now: i64) -> Res<Item> {
        let conn = self.conn();
        let mut item = Self::get(&conn, id)?;
        item.status = "open".into();
        item.completed_at = None;
        item.updated_at = now;
        upsert(&conn, &item).map_err(err)?;
        Ok(item)
    }

    pub fn snooze(&self, id: &str, until: i64, now: i64) -> Res<Item> {
        let conn = self.conn();
        let mut item = Self::get(&conn, id)?;
        item.snoozed_until = Some(until);
        item.last_fired_at = None;
        item.last_nag_at = None;
        item.updated_at = now;
        upsert(&conn, &item).map_err(err)?;
        Ok(item)
    }

    /// Re-order the given items among themselves: they take over their own set of
    /// `sort_order` slots in the new order, so they stay where they were relative
    /// to everything else.
    pub fn reorder(&self, ids: &[String]) -> Res<()> {
        let mut conn = self.conn();
        let tx = conn.transaction().map_err(err)?;
        let mut slots = Vec::with_capacity(ids.len());
        for id in ids {
            let o: f64 = tx
                .query_row("SELECT sort_order FROM items WHERE id = ?1", [id], |r| r.get(0))
                .map_err(err)?;
            slots.push(o);
        }
        slots.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        // Make slots unique so a tie can never leave the order undefined.
        for k in 1..slots.len() {
            if slots[k] <= slots[k - 1] {
                slots[k] = slots[k - 1] + 0.001;
            }
        }
        for (id, slot) in ids.iter().zip(slots) {
            tx.execute("UPDATE items SET sort_order = ?1 WHERE id = ?2", params![slot, id])
                .map_err(err)?;
        }
        tx.commit().map_err(err)
    }

    pub fn license_get(&self) -> Res<Option<Entitlement>> {
        self.conn()
            .query_row(
                "SELECT key, instance_id, validated_at, customer FROM license WHERE id = 1",
                [],
                |r| {
                    Ok(Entitlement {
                        key: r.get(0)?,
                        instance_id: r.get(1)?,
                        validated_at: r.get(2)?,
                        customer: r.get(3)?,
                    })
                },
            )
            .optional()
            .map_err(err)
    }

    pub fn license_set(&self, e: &Entitlement) -> Res<()> {
        self.conn()
            .execute(
                "INSERT OR REPLACE INTO license (id, key, instance_id, validated_at, customer) VALUES (1, ?1, ?2, ?3, ?4)",
                params![e.key, e.instance_id, e.validated_at, e.customer],
            )
            .map_err(err)?;
        Ok(())
    }

    pub fn license_clear(&self) -> Res<()> {
        self.conn().execute("DELETE FROM license", []).map_err(err)?;
        Ok(())
    }

    pub fn settings(&self) -> Res<BTreeMap<String, String>> {
        let conn = self.conn();
        let mut map: BTreeMap<String, String> = DEFAULT_SETTINGS
            .iter()
            .map(|(k, v)| ((*k).to_string(), (*v).to_string()))
            .collect();
        let mut stmt = conn.prepare("SELECT key, value FROM settings").map_err(err)?;
        for row in stmt
            .query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)))
            .map_err(err)?
        {
            let (k, v) = row.map_err(err)?;
            map.insert(k, v);
        }
        Ok(map)
    }

    pub fn set_setting(&self, key: &str, value: &str) -> Res<()> {
        self.conn()
            .execute(
                "INSERT INTO settings (key, value) VALUES (?1, ?2) \
                 ON CONFLICT(key) DO UPDATE SET value = excluded.value",
                params![key, value],
            )
            .map_err(err)?;
        Ok(())
    }

    /// Delete completed items older than `days` (0 = keep forever). Returns the count.
    pub fn purge_completed(&self, days: i64, now: i64) -> Res<usize> {
        if days <= 0 {
            return Ok(0);
        }
        let cutoff = now - days * 86_400_000;
        self.conn()
            .execute(
                "DELETE FROM items WHERE status = 'done' AND COALESCE(completed_at, 0) < ?1",
                [cutoff],
            )
            .map_err(err)
    }

    pub fn export(&self, now: i64) -> Res<ExportFile> {
        let items = self.list(now)?;
        Ok(ExportFile {
            version: 1,
            exported_at: now,
            items,
            settings: self.settings()?,
        })
    }

    /// `replace`: wipe items and settings first. Otherwise merge by id; an existing
    /// item is only overwritten by a strictly newer `updated_at`.
    pub fn import(&self, file: &ExportFile, replace: bool) -> Res<ImportResult> {
        if file.version != 1 {
            return Err(format!("unsupported export version {}", file.version));
        }
        let mut conn = self.conn();
        let tx = conn.transaction().map_err(err)?;
        let mut result = ImportResult::default();
        if replace {
            tx.execute("DELETE FROM items", []).map_err(err)?;
            tx.execute("DELETE FROM settings", []).map_err(err)?;
        }
        for item in &file.items {
            let existing: Option<i64> = tx
                .query_row("SELECT updated_at FROM items WHERE id = ?1", [&item.id], |r| r.get(0))
                .optional()
                .map_err(err)?;
            match existing {
                None => {
                    upsert(&tx, item).map_err(err)?;
                    result.added += 1;
                }
                Some(t) if item.updated_at > t => {
                    upsert(&tx, item).map_err(err)?;
                    result.updated += 1;
                }
                Some(_) => result.skipped += 1,
            }
        }
        if replace {
            for (k, v) in &file.settings {
                tx.execute(
                    "INSERT OR REPLACE INTO settings (key, value) VALUES (?1, ?2)",
                    params![k, v],
                )
                .map_err(err)?;
            }
        }
        tx.commit().map_err(err)?;
        Ok(result)
    }
}

/// The anchor day is only meaningful for monthly items that have a due date.
fn anchor_for(repeat: &str, due_at: Option<i64>) -> Option<i64> {
    if repeat != "monthly" {
        return None;
    }
    due_at.and_then(repeat::local_day).map(i64::from)
}

/// `PRAGMA user_version` migrations.
fn migrate(conn: &Connection) -> Res<()> {
    let version: i64 = conn
        .pragma_query_value(None, "user_version", |r| r.get(0))
        .map_err(err)?;
    if version < 1 {
        conn.execute_batch(&format!("BEGIN; {SCHEMA_V1} PRAGMA user_version = 1; COMMIT;"))
            .map_err(err)?;
    }
    if version < 2 {
        conn.execute_batch(&format!("BEGIN; {SCHEMA_V2} PRAGMA user_version = 2; COMMIT;"))
            .map_err(err)?;
    }
    if version < 3 {
        conn.execute_batch(&format!("BEGIN; {SCHEMA_V3} PRAGMA user_version = 3; COMMIT;"))
            .map_err(err)?;
    }
    if version < 4 {
        conn.execute_batch(&format!("BEGIN; {SCHEMA_V4} PRAGMA user_version = 4; COMMIT;"))
            .map_err(err)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn edit(title: &str) -> ItemEdit {
        ItemEdit {
            title: title.into(),
            repeat: "none".into(),
            priority: 1,
            ..Default::default()
        }
    }

    #[test]
    fn crud_round_trip() {
        let db = Db::open_memory().unwrap();
        let a = db.create(&edit("  Release 2.3 "), 1000).unwrap();
        assert_eq!(a.title, "Release 2.3");
        let mut e = edit("Release 2.4");
        e.tag = Some("  api ".into());
        e.link = Some("   ".into());
        e.due_at = Some(5000);
        let u = db.update(&a.id, &e, 2000).unwrap();
        assert_eq!(u.tag.as_deref(), Some("api"));
        assert_eq!(u.link, None);
        assert_eq!(db.list(0).unwrap().len(), 1);
        db.delete(&a.id).unwrap();
        assert!(db.list(0).unwrap().is_empty());
        db.undelete(&u).unwrap();
        assert_eq!(db.list(0).unwrap()[0], u);
    }

    #[test]
    fn complete_and_restore() {
        let db = Db::open_memory().unwrap();
        let a = db.create(&edit("one"), 1).unwrap();
        let done = db.complete(&a.id, 50).unwrap();
        assert!(done.is_done());
        assert_eq!(done.completed_at, Some(50));
        let back = db.restore(&a.id, 60).unwrap();
        assert!(!back.is_done());
        assert_eq!(back.completed_at, None);
    }

    #[test]
    fn completing_a_repeating_item_rolls_it_forward() {
        let db = Db::open_memory().unwrap();
        let mut e = edit("standup");
        e.repeat = "daily".into();
        e.due_at = Some(1_700_000_000_000);
        let a = db.create(&e, 1).unwrap();
        db.snooze(&a.id, 1_700_000_100_000, 2).unwrap();
        let rolled = db.complete(&a.id, 1_700_000_000_000).unwrap();
        assert_eq!(rolled.status, "open");
        assert!(rolled.due_at.unwrap() > 1_700_000_000_000);
        assert_eq!(rolled.snoozed_until, None);
        assert_eq!(rolled.completed_at, None);
    }

    #[test]
    fn purge_removes_only_old_completed() {
        let db = Db::open_memory().unwrap();
        let day = 86_400_000;
        let old = db.create(&edit("old"), 0).unwrap();
        let recent = db.create(&edit("recent"), 0).unwrap();
        let open = db.create(&edit("open"), 0).unwrap();
        db.complete(&old.id, 0).unwrap();
        db.complete(&recent.id, 9 * day).unwrap();
        assert_eq!(db.purge_completed(7, 10 * day).unwrap(), 1);
        let ids: Vec<_> = db.list(10 * day).unwrap().into_iter().map(|i| i.id).collect();
        assert!(ids.contains(&recent.id) && ids.contains(&open.id) && !ids.contains(&old.id));
        assert_eq!(db.purge_completed(0, 100 * day).unwrap(), 0);
    }

    #[test]
    fn reorder_swaps_slots_among_given_items() {
        let db = Db::open_memory().unwrap();
        let a = db.create(&edit("a"), 1).unwrap();
        let b = db.create(&edit("b"), 2).unwrap();
        let c = db.create(&edit("c"), 3).unwrap();
        db.reorder(&[c.id.clone(), a.id.clone(), b.id.clone()]).unwrap();
        let titles: Vec<_> = db.list(0).unwrap().into_iter().map(|i| i.title).collect();
        assert_eq!(titles, ["c", "a", "b"]);
    }

    #[test]
    fn export_import_replace_is_lossless() {
        let src = Db::open_memory().unwrap();
        let mut e = edit("Ship it");
        e.body_md = "- [ ] one\n- [x] two\n".into();
        e.due_at = Some(1_800_000_000_000);
        e.tag = Some("api".into());
        e.link = Some("https://example.com".into());
        e.branch = Some("release/2.3".into());
        e.repeat = "weekly".into();
        e.priority = 2;
        let a = src.create(&e, 10).unwrap();
        src.create(&edit("plain"), 11).unwrap();
        let done = src.create(&edit("done one"), 12).unwrap();
        src.complete(&done.id, 20).unwrap();
        src.snooze(&a.id, 1_800_000_500_000, 21).unwrap();
        src.set_setting("autoclear_days", "3").unwrap();

        let json = serde_json::to_string(&src.export(100).unwrap()).unwrap();
        let dst = Db::open_memory().unwrap();
        dst.create(&edit("will be wiped"), 1).unwrap();
        let file: ExportFile = serde_json::from_str(&json).unwrap();
        dst.import(&file, true).unwrap();

        assert_eq!(src.list(100).unwrap(), dst.list(100).unwrap());
        assert_eq!(src.settings().unwrap(), dst.settings().unwrap());
    }

    #[test]
    fn merge_only_takes_newer_items() {
        let db = Db::open_memory().unwrap();
        let a = db.create(&edit("mine"), 100).unwrap();
        let mut older = a.clone();
        older.title = "older".into();
        older.updated_at = 50;
        let mut newer = a.clone();
        newer.title = "newer".into();
        newer.updated_at = 200;
        let mut extra = a.clone();
        extra.id = "x".into();
        let mk = |items: Vec<Item>| ExportFile {
            version: 1,
            exported_at: 0,
            items,
            settings: Default::default(),
        };
        let r = db.import(&mk(vec![older]), false).unwrap();
        assert_eq!((r.skipped, r.updated, r.added), (1, 0, 0));
        let r = db.import(&mk(vec![newer, extra]), false).unwrap();
        assert_eq!((r.skipped, r.updated, r.added), (0, 1, 1));
        assert_eq!(db.list(0).unwrap().len(), 2);
        assert!(db.list(0).unwrap().iter().any(|i| i.title == "newer"));
    }

    #[test]
    fn changing_due_time_rearms_the_reminder() {
        let db = Db::open_memory().unwrap();
        let mut e = edit("x");
        e.due_at = Some(100);
        let a = db.create(&e, 1).unwrap();
        db.snooze(&a.id, 500, 2).unwrap();
        e.due_at = Some(900);
        let u = db.update(&a.id, &e, 3).unwrap();
        assert_eq!(u.snoozed_until, None);
    }

    #[test]
    fn monthly_item_keeps_its_anchor_day_across_completions() {
        use chrono::{Local, TimeZone};
        let ms = |y, m, d| Local.with_ymd_and_hms(y, m, d, 8, 0, 0).unwrap().timestamp_millis();
        let db = Db::open_memory().unwrap();
        let mut e = edit("rotate keys");
        e.repeat = "monthly".into();
        e.due_at = Some(ms(2026, 1, 31));
        let a = db.create(&e, 1).unwrap();
        assert_eq!(a.repeat_anchor_day, Some(31));
        let feb = db.complete(&a.id, ms(2026, 1, 31)).unwrap();
        assert_eq!(feb.due_at, Some(ms(2026, 2, 28)));
        let mar = db.complete(&a.id, ms(2026, 2, 28)).unwrap();
        assert_eq!(mar.due_at, Some(ms(2026, 3, 31)));
        // Editing the due date re-anchors; leaving monthly clears the anchor.
        e.due_at = Some(ms(2026, 3, 15));
        assert_eq!(db.update(&a.id, &e, 5).unwrap().repeat_anchor_day, Some(15));
        e.repeat = "weekly".into();
        assert_eq!(db.update(&a.id, &e, 6).unwrap().repeat_anchor_day, None);
    }

    #[test]
    fn migration_v2_adds_anchor_column_to_a_v1_database() {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(&format!("{SCHEMA_V1} PRAGMA user_version = 1;")).unwrap();
        conn.execute("INSERT INTO items (id,title,sort_order,created_at,updated_at) VALUES ('a', 't', 1, 1, 1)", []).unwrap();
        let db = Db::init(conn).unwrap();
        assert_eq!(db.list(0).unwrap()[0].repeat_anchor_day, None);
    }

    #[test]
    fn migrates_a_v2_database_to_the_current_schema_keeping_items() {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(&format!("{SCHEMA_V1} {SCHEMA_V2} PRAGMA user_version = 2;")).unwrap();
        conn.execute(
            "INSERT INTO items (id, title, sort_order, created_at, updated_at, due_at) VALUES ('a', 'old', 1, 1, 1, 5000)",
            [],
        )
        .unwrap();
        let db = Db::init(conn).unwrap();
        let items = db.list(0).unwrap();
        assert_eq!((items[0].title.as_str(), items[0].prealert_min, items[0].last_nag_at), ("old", None, None));
        assert_eq!(db.license_get().unwrap(), None);
        let v: i64 = db.conn().pragma_query_value(None, "user_version", |r| r.get(0)).unwrap();
        assert_eq!(v, 4);
    }

    #[test]
    fn prealert_and_nag_markers_re_arm_with_the_item() {
        let db = Db::open_memory().unwrap();
        let mut e = edit("x");
        e.due_at = Some(100_000);
        e.prealert_min = Some(15);
        e.priority = 3;
        let it = db.create(&e, 1).unwrap();
        assert_eq!((it.prealert_min, it.priority), (Some(15), 3));
        // Unsupported lengths are dropped.
        let mut bad = edit("y");
        bad.prealert_min = Some(7);
        assert_eq!(db.create(&bad, 1).unwrap().prealert_min, None);

        db.mark_pre_fired(std::slice::from_ref(&it.id), 50).unwrap();
        db.mark_nagged(std::slice::from_ref(&it.id), 60).unwrap();
        let got = &db.list(0).unwrap().into_iter().find(|i| i.id == it.id).unwrap();
        assert_eq!((got.pre_fired_at, got.last_nag_at), (Some(50), Some(60)));
        // Snooze keeps pre_fired but clears the nag clock.
        let s = db.snooze(&it.id, 90_000, 2).unwrap();
        assert_eq!((s.pre_fired_at, s.last_nag_at), (Some(50), None));
        // Changing the pre-alert length re-arms it; changing the due time re-arms everything.
        e.prealert_min = Some(30);
        assert_eq!(db.update(&it.id, &e, 3).unwrap().pre_fired_at, None);
        db.mark_pre_fired(std::slice::from_ref(&it.id), 70).unwrap();
        e.due_at = Some(500_000);
        let u = db.update(&it.id, &e, 4).unwrap();
        assert_eq!((u.pre_fired_at, u.last_nag_at, u.last_fired_at), (None, None, None));
        // Completing a repeating item re-arms for the next occurrence.
        e.repeat = "daily".into();
        db.update(&it.id, &e, 5).unwrap();
        db.mark_pre_fired(std::slice::from_ref(&it.id), 80).unwrap();
        let c = db.complete(&it.id, 6).unwrap();
        assert_eq!(c.pre_fired_at, None);
    }

    #[test]
    fn gate_edit_keeps_pro_fields_out_of_free_builds() {
        use crate::model::gate_edit;
        let mut e = edit("x");
        e.priority = 3;
        e.prealert_min = Some(15);
        gate_edit(&mut e, None, false);
        assert_eq!((e.priority, e.prealert_min), (2, None));
        // An item that is already critical / has a pre-alert keeps them through unrelated edits.
        let mut existing = Item::blank("i");
        existing.priority = 3;
        existing.prealert_min = Some(30);
        let mut e = edit("x");
        e.priority = 3;
        e.prealert_min = Some(5);
        gate_edit(&mut e, Some(&existing), false);
        assert_eq!((e.priority, e.prealert_min), (3, Some(30)));
        let mut e = edit("x");
        e.priority = 3;
        gate_edit(&mut e, None, true);
        assert_eq!(e.priority, 3);
    }
}
