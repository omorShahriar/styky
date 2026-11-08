//! Styky Pro licensing against the Lemon Squeezy License API (public endpoints, no secret key).
//!
//! * `activate` binds a key to this PC (`instance_name` = "<PC name> Styky"); the entitlement
//!   (key, instance id, last successful validation) is cached in the `license` table.
//! * Pro is on while the last successful validation is at most [`GRACE_MS`] (30 days) old.
//! * [`revalidate_due`] re-checks after [`REVALIDATE_MS`] (7 days). **Only an explicit rejection**
//!   (`invalid` / `disabled` / `expired` / unknown key / unknown instance) removes the entitlement;
//!   network errors, timeouts and 5xx keep it, so Pro never breaks offline.
//! * The HTTP layer is the [`Http`] trait so the state machine is unit-tested with a mock.

use std::sync::atomic::{AtomicBool, Ordering};

use serde::{Deserialize, Serialize};

use crate::db::{Db, Res};

// ---- configuration (owner: fill these in from the Lemon Squeezy dashboard) ------------------

/// Checkout page opened by the "Buy" button.
pub const STYKY_BUY_URL: &str = "https://styky.lemonsqueezy.com/buy/REPLACE-WITH-CHECKOUT-ID";
/// Lemon Squeezy store id and product id a key must belong to. `0` = not configured yet: the
/// check is skipped (anything Lemon issued is accepted). Set both before selling keys.
pub const LEMON_STORE_ID: u64 = 0;
pub const LEMON_PRODUCT_ID: u64 = 0;

const API: &str = "https://api.lemonsqueezy.com/v1/licenses";
pub const REVALIDATE_MS: i64 = 7 * 86_400_000;
pub const GRACE_MS: i64 = 30 * 86_400_000;

static PRO: AtomicBool = AtomicBool::new(false);

/// Debug builds only: `STYKY_PRO_DEV=1` forces Pro on (for testing without a key).
pub fn dev_forced() -> bool {
    cfg!(debug_assertions) && std::env::var("STYKY_PRO_DEV").is_ok_and(|v| v == "1")
}

/// Is Pro unlocked right now? Cheap; safe to call from any thread.
pub fn is_pro() -> bool {
    dev_forced() || PRO.load(Ordering::SeqCst)
}

// ---- model ----------------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Entitlement {
    pub key: String,
    pub instance_id: String,
    pub validated_at: i64,
    pub customer: Option<String>,
}

/// Pure: does this entitlement unlock Pro at `now`?
pub fn entitles(e: Option<&Entitlement>, now: i64) -> bool {
    e.is_some_and(|e| now - e.validated_at <= GRACE_MS)
}

/// Pure: is a background re-check due?
pub fn needs_revalidate(e: &Entitlement, now: i64) -> bool {
    now - e.validated_at >= REVALIDATE_MS || e.validated_at > now
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Status {
    pub pro: bool,
    /// `none` | `active` | `grace-expired` (cached but not validated for 30+ days) | `dev`.
    pub state: &'static str,
    pub key_hint: Option<String>,
    pub customer: Option<String>,
    pub validated_at: Option<i64>,
    pub buy_url: &'static str,
}

pub fn status(db: &Db, now: i64) -> Status {
    let ent = db.license_get().ok().flatten();
    let entitled = entitles(ent.as_ref(), now);
    let state = if dev_forced() {
        "dev"
    } else if entitled {
        "active"
    } else if ent.is_some() {
        "grace-expired"
    } else {
        "none"
    };
    Status {
        pro: dev_forced() || entitled,
        state,
        key_hint: ent.as_ref().map(|e| hint(&e.key)),
        customer: ent.as_ref().and_then(|e| e.customer.clone()),
        validated_at: ent.as_ref().map(|e| e.validated_at),
        buy_url: STYKY_BUY_URL,
    }
}

/// `XXXX-…-1234`: never show the whole key again.
fn hint(key: &str) -> String {
    let tail: String = key.chars().rev().take(4).collect::<Vec<_>>().into_iter().rev().collect();
    format!("…{tail}")
}

/// Recompute the in-memory flag from the DB; returns true when it changed.
pub fn refresh(db: &Db, now: i64) -> bool {
    let ent = db.license_get().ok().flatten();
    let new = entitles(ent.as_ref(), now);
    PRO.swap(new, Ordering::SeqCst) != new
}

// ---- HTTP layer -----------------------------------------------------------------------------

pub struct HttpResponse {
    pub status: u16,
    pub body: String,
}

/// POST a form to the license API. `Err` = the request never completed (offline, DNS, timeout…).
pub trait Http {
    fn post_form(&self, url: &str, form: &[(&str, &str)]) -> Result<HttpResponse, String>;
}

pub struct UreqHttp;

impl Http for UreqHttp {
    fn post_form(&self, url: &str, form: &[(&str, &str)]) -> Result<HttpResponse, String> {
        let agent: ureq::Agent = ureq::Agent::config_builder()
            .timeout_global(Some(std::time::Duration::from_secs(15)))
            .http_status_as_error(false)
            .build()
            .into();
        let mut resp = agent
            .post(url)
            .header("Accept", "application/json")
            .send_form(form.iter().copied())
            .map_err(|e| e.to_string())?;
        let status = resp.status().as_u16();
        let body = resp.body_mut().read_to_string().map_err(|e| e.to_string())?;
        Ok(HttpResponse { status, body })
    }
}

// ---- API calls ------------------------------------------------------------------------------

#[derive(Deserialize, Default)]
struct ApiResp {
    activated: Option<bool>,
    valid: Option<bool>,
    deactivated: Option<bool>,
    error: Option<String>,
    license_key: Option<ApiKey>,
    instance: Option<ApiInstance>,
    meta: Option<ApiMeta>,
}
#[derive(Deserialize)]
struct ApiKey {
    status: Option<String>,
}
#[derive(Deserialize)]
struct ApiInstance {
    id: String,
}
#[derive(Deserialize)]
struct ApiMeta {
    store_id: Option<u64>,
    product_id: Option<u64>,
    customer_name: Option<String>,
}

/// What the API said, reduced to what the state machine needs.
#[derive(Debug, PartialEq)]
enum Outcome {
    Ok { instance_id: Option<String>, customer: Option<String> },
    /// An explicit "no" from Lemon Squeezy (message for the UI).
    Rejected(String),
    WrongProduct,
    /// Could not get a trustworthy answer (offline, 5xx, rate limit, garbage). Never removes Pro.
    Unavailable(String),
}

fn product_ok(meta: &Option<ApiMeta>) -> bool {
    product_matches(meta, LEMON_STORE_ID, LEMON_PRODUCT_ID)
}

/// `0` for both ids = unconfigured: accept. Otherwise every configured id must match.
fn product_matches(meta: &Option<ApiMeta>, store: u64, product: u64) -> bool {
    if store == 0 && product == 0 {
        return true;
    }
    let Some(m) = meta else { return false };
    (store == 0 || m.store_id == Some(store)) && (product == 0 || m.product_id == Some(product))
}

#[derive(Clone, Copy, PartialEq)]
enum Call {
    Activate,
    Validate,
    Deactivate,
}

fn call(http: &dyn Http, what: Call, form: &[(&str, &str)]) -> Outcome {
    let path = match what {
        Call::Activate => "activate",
        Call::Validate => "validate",
        Call::Deactivate => "deactivate",
    };
    let resp = match http.post_form(&format!("{API}/{path}"), form) {
        Ok(r) => r,
        Err(e) => return Outcome::Unavailable(e),
    };
    if resp.status >= 500 || resp.status == 429 || resp.status == 408 {
        return Outcome::Unavailable(format!("license server returned {}", resp.status));
    }
    let Ok(body) = serde_json::from_str::<ApiResp>(&resp.body) else {
        return Outcome::Unavailable(format!("unreadable response ({})", resp.status));
    };
    let key_status = body.license_key.as_ref().and_then(|k| k.status.as_deref()).unwrap_or("");
    let customer = body.meta.as_ref().and_then(|m| m.customer_name.clone());
    let ok = match what {
        Call::Activate => body.activated == Some(true),
        Call::Validate => body.valid == Some(true) && !matches!(key_status, "expired" | "disabled" | "invalid"),
        Call::Deactivate => body.deactivated == Some(true),
    };
    if ok {
        if what != Call::Deactivate && !product_ok(&body.meta) {
            return Outcome::WrongProduct;
        }
        return Outcome::Ok { instance_id: body.instance.map(|i| i.id), customer };
    }
    // A 2xx/4xx that is not "ok" is an explicit answer.
    let msg = body.error.unwrap_or_else(|| match key_status {
        "expired" => "This license has expired.".into(),
        "disabled" => "This license has been disabled.".into(),
        _ => "This license key is not valid.".into(),
    });
    Outcome::Rejected(msg)
}

pub fn instance_name() -> String {
    let pc = std::env::var("COMPUTERNAME").unwrap_or_else(|_| "PC".into());
    format!("{pc} Styky")
}

/// Activate `key` on this PC and cache the entitlement.
pub fn activate(db: &Db, http: &dyn Http, key: &str, name: &str, now: i64) -> Res<Entitlement> {
    let key = key.trim();
    if key.is_empty() {
        return Err("Enter your license key.".into());
    }
    match call(http, Call::Activate, &[("license_key", key), ("instance_name", name)]) {
        Outcome::Ok { instance_id: Some(instance_id), customer } => {
            let ent = Entitlement { key: key.to_string(), instance_id, validated_at: now, customer };
            db.license_set(&ent)?;
            Ok(ent)
        }
        Outcome::Ok { .. } => Err("The license server did not return an activation.".into()),
        Outcome::Rejected(m) => Err(m),
        Outcome::WrongProduct => Err("This key is for a different product.".into()),
        Outcome::Unavailable(e) => Err(format!("Could not reach the license server ({e}). Check your connection and try again.")),
    }
}

/// Result of a background re-check.
#[derive(Debug, PartialEq)]
pub enum Revalidation {
    /// Nothing cached, or not due yet.
    Skipped,
    Renewed,
    /// Explicitly rejected: the entitlement was removed.
    Revoked,
    /// Offline / server trouble: the cached entitlement is untouched.
    Kept,
}

/// Re-check the cached entitlement if it is due (or `force`).
pub fn revalidate(db: &Db, http: &dyn Http, now: i64, force: bool) -> Revalidation {
    let Ok(Some(ent)) = db.license_get() else { return Revalidation::Skipped };
    if !force && !needs_revalidate(&ent, now) {
        return Revalidation::Skipped;
    }
    match call(http, Call::Validate, &[("license_key", &ent.key), ("instance_id", &ent.instance_id)]) {
        Outcome::Ok { customer, .. } => {
            let _ = db.license_set(&Entitlement { validated_at: now, customer: customer.or(ent.customer), ..ent });
            Revalidation::Renewed
        }
        Outcome::Rejected(_) | Outcome::WrongProduct => {
            let _ = db.license_clear();
            Revalidation::Revoked
        }
        Outcome::Unavailable(_) => Revalidation::Kept,
    }
}

/// "Remove license": free the activation slot (best effort) and forget the key locally.
/// Offline removal still succeeds locally; the slot can be freed from the Lemon Squeezy customer portal.
pub fn deactivate(db: &Db, http: &dyn Http) -> Res<bool> {
    let Some(ent) = db.license_get()? else { return Ok(true) };
    let freed = matches!(
        call(http, Call::Deactivate, &[("license_key", &ent.key), ("instance_id", &ent.instance_id)]),
        Outcome::Ok { .. }
    );
    db.license_clear()?;
    Ok(freed)
}

/// Startup / periodic check, called from a background thread.
pub fn background_check(app: &tauri::AppHandle) {
    use tauri::{Emitter, Manager};
    let db = app.state::<Db>();
    let now = crate::commands::now_ms();
    let before = is_pro();
    revalidate(&db, &UreqHttp, now, false);
    refresh(&db, now);
    if before != is_pro() {
        let _ = app.emit("pro-changed", is_pro());
    }
}

// ---- tests ----------------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;

    type Recorded = (String, Vec<(String, String)>);

    /// Scripted responses, recording the calls made.
    struct Mock {
        replies: RefCell<Vec<Result<HttpResponse, String>>>,
        calls: RefCell<Vec<Recorded>>,
    }
    impl Mock {
        fn new(replies: Vec<Result<HttpResponse, String>>) -> Self {
            Self { replies: RefCell::new(replies), calls: RefCell::new(vec![]) }
        }
    }
    impl Http for Mock {
        fn post_form(&self, url: &str, form: &[(&str, &str)]) -> Result<HttpResponse, String> {
            self.calls
                .borrow_mut()
                .push((url.to_string(), form.iter().map(|(a, b)| (a.to_string(), b.to_string())).collect()));
            self.replies.borrow_mut().remove(0)
        }
    }
    fn reply(status: u16, body: &str) -> Result<HttpResponse, String> {
        Ok(HttpResponse { status, body: body.into() })
    }
    const ACTIVATED: &str = r#"{"activated":true,"error":null,"license_key":{"status":"active"},"instance":{"id":"inst-1","name":"X"},"meta":{"store_id":1,"product_id":2,"customer_name":"Ada"}}"#;
    const VALID: &str = r#"{"valid":true,"error":null,"license_key":{"status":"active"},"instance":{"id":"inst-1"},"meta":{"store_id":1,"product_id":2,"customer_name":"Ada"}}"#;
    const DAY: i64 = 86_400_000;

    fn activated_db(at: i64) -> Db {
        let db = Db::open_memory().unwrap();
        let http = Mock::new(vec![reply(200, ACTIVATED)]);
        activate(&db, &http, "KEY-1234", "PC Styky", at).unwrap();
        db
    }

    #[test]
    fn activate_caches_the_entitlement_and_sends_the_instance_name() {
        let db = Db::open_memory().unwrap();
        let http = Mock::new(vec![reply(200, ACTIVATED)]);
        let e = activate(&db, &http, "  KEY-1234 ", "DESK Styky", 1000).unwrap();
        assert_eq!((e.instance_id.as_str(), e.validated_at, e.customer.as_deref()), ("inst-1", 1000, Some("Ada")));
        assert_eq!(db.license_get().unwrap(), Some(e));
        let calls = http.calls.borrow();
        assert!(calls[0].0.ends_with("/licenses/activate"));
        assert!(calls[0].1.contains(&("license_key".into(), "KEY-1234".into())));
        assert!(calls[0].1.contains(&("instance_name".into(), "DESK Styky".into())));
    }

    #[test]
    fn activate_reports_rejections_and_offline_without_caching() {
        let db = Db::open_memory().unwrap();
        let bad = Mock::new(vec![reply(404, r#"{"activated":false,"error":"license_key not found."}"#)]);
        assert_eq!(activate(&db, &bad, "nope", "n", 0).unwrap_err(), "license_key not found.");
        let limit = Mock::new(vec![reply(400, r#"{"activated":false,"error":"This license key has reached the activation limit."}"#)]);
        assert!(activate(&db, &limit, "k", "n", 0).unwrap_err().contains("activation limit"));
        let off = Mock::new(vec![Err("dns error".into())]);
        assert!(activate(&db, &off, "k", "n", 0).unwrap_err().contains("Could not reach"));
        let down = Mock::new(vec![reply(503, "<html>")]);
        assert!(activate(&db, &down, "k", "n", 0).unwrap_err().contains("Could not reach"));
        assert!(activate(&db, &bad, "  ", "n", 0).is_err());
        assert_eq!(db.license_get().unwrap(), None);
    }

    #[test]
    fn grace_window_is_30_days_from_the_last_validation() {
        let e = Entitlement { key: "k".into(), instance_id: "i".into(), validated_at: 0, customer: None };
        assert!(entitles(Some(&e), 29 * DAY));
        assert!(entitles(Some(&e), 30 * DAY));
        assert!(!entitles(Some(&e), 30 * DAY + 1));
        assert!(!entitles(None, 0));
        assert!(!needs_revalidate(&e, 6 * DAY));
        assert!(needs_revalidate(&e, 7 * DAY));
    }

    #[test]
    fn revalidation_skips_until_due_then_renews() {
        let db = activated_db(0);
        let http = Mock::new(vec![reply(200, VALID)]);
        assert_eq!(revalidate(&db, &http, 3 * DAY, false), Revalidation::Skipped);
        assert!(http.calls.borrow().is_empty());
        assert_eq!(revalidate(&db, &http, 8 * DAY, false), Revalidation::Renewed);
        assert_eq!(db.license_get().unwrap().unwrap().validated_at, 8 * DAY);
        let c = http.calls.borrow();
        assert!(c[0].0.ends_with("/licenses/validate"));
        assert!(c[0].1.contains(&("instance_id".into(), "inst-1".into())));
    }

    #[test]
    fn offline_or_server_errors_never_remove_pro() {
        let db = activated_db(0);
        for r in [Err("timeout".to_string()), reply(500, "oops"), reply(429, "{}"), reply(200, "not json")] {
            assert_eq!(revalidate(&db, &Mock::new(vec![r]), 10 * DAY, false), Revalidation::Kept);
            assert!(db.license_get().unwrap().is_some());
        }
        // Still entitled inside the grace window, even though the re-check keeps failing.
        let e = db.license_get().unwrap();
        assert!(entitles(e.as_ref(), 29 * DAY));
        // Past 30 days offline Pro pauses, but the key is kept and recovers once online.
        assert!(!entitles(e.as_ref(), 31 * DAY));
        assert_eq!(revalidate(&db, &Mock::new(vec![reply(200, VALID)]), 31 * DAY, false), Revalidation::Renewed);
        assert!(entitles(db.license_get().unwrap().as_ref(), 40 * DAY));
    }

    #[test]
    fn explicit_invalid_disabled_expired_remove_pro() {
        let cases = [
            r#"{"valid":false,"error":"license_key not found.","license_key":null}"#,
            r#"{"valid":false,"license_key":{"status":"disabled"}}"#,
            r#"{"valid":false,"license_key":{"status":"expired"}}"#,
            r#"{"valid":true,"license_key":{"status":"expired"}}"#,
            r#"{"valid":false,"error":"instance not found"}"#,
        ];
        for body in cases {
            let db = activated_db(0);
            let r = revalidate(&db, &Mock::new(vec![reply(400, body)]), 8 * DAY, false);
            assert_eq!(r, Revalidation::Revoked, "{body}");
            assert_eq!(db.license_get().unwrap(), None);
        }
    }

    #[test]
    fn remove_license_deactivates_and_forgets_even_when_offline() {
        let db = activated_db(0);
        let http = Mock::new(vec![reply(200, r#"{"deactivated":true,"error":null}"#)]);
        assert!(deactivate(&db, &http).unwrap());
        assert!(http.calls.borrow()[0].0.ends_with("/licenses/deactivate"));
        assert_eq!(db.license_get().unwrap(), None);
        let db = activated_db(0);
        assert!(!deactivate(&db, &Mock::new(vec![Err("offline".into())])).unwrap());
        assert_eq!(db.license_get().unwrap(), None);
    }

    #[test]
    fn status_reflects_the_state_machine() {
        let db = Db::open_memory().unwrap();
        assert_eq!(status(&db, 0).state, "none");
        let db = activated_db(0);
        let s = status(&db, DAY);
        assert_eq!((s.pro, s.state, s.key_hint.as_deref()), (true, "active", Some("…1234")));
        let s = status(&db, 31 * DAY);
        assert_eq!((s.pro, s.state), (false, "grace-expired"));
    }

    #[test]
    fn product_check_rejects_keys_from_other_stores() {
        let m = |s, p| Some(ApiMeta { store_id: Some(s), product_id: Some(p), customer_name: None });
        assert!(product_matches(&m(1, 2), 1, 2));
        assert!(!product_matches(&m(9, 2), 1, 2));
        assert!(!product_matches(&m(1, 9), 1, 2));
        assert!(!product_matches(&None, 1, 2));
        // Unconfigured placeholders accept everything.
        assert!(product_matches(&m(9, 9), 0, 0));
    }
}
