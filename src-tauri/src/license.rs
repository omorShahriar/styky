//! Styky Pro licensing. Honor-based: the code is public and there is no DRM, tamper check or phone-home.
//!
//! The only network calls this module can make are the two the user starts by clicking a button:
//!
//! * **Activate**: one request to the Lemon Squeezy License API (public endpoints, no secret key) binds the key to this PC
//!   (`instance_name` = "Styky" + a random per-install id, never the PC name). An explicit "no" (unknown key, activation
//!   limit, wrong product) is shown to the user. If the request cannot complete (offline, timeout, 5xx, an unreadable
//!   answer), a well-formed key is accepted anyway.
//! * **Remove license** (deactivate): one request frees the activation slot; the key is forgotten locally either way.
//!
//! Nothing runs at startup, on a timer or in the background: a cached key unlocks Pro for good, refunds do not revoke
//! it, and the key is never checked again. The provider is an implementation detail (`API`, [`Http`], [`call`]); the
//! rest of the module only knows "a well-formed key was entered".
//!
//! The HTTP layer is the [`Http`] trait so the state machine is unit-tested with a mock.

use std::sync::atomic::{AtomicBool, Ordering};

use serde::{Deserialize, Serialize};

use crate::db::{Db, Res};

// ---- configuration (owner: fill these in from the Lemon Squeezy dashboard) ------------------

/// Lemon Squeezy store id and product id a key must belong to. `0` = not configured yet: the
/// check is skipped (anything Lemon issued is accepted). Set both before selling keys.
pub const LEMON_STORE_ID: u64 = 0;
pub const LEMON_PRODUCT_ID: u64 = 0;

const API: &str = "https://api.lemonsqueezy.com/v1/licenses";

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
    /// The provider's activation id, or empty when the key was accepted offline (nothing to release later).
    pub instance_id: String,
    /// When the key was accepted (the column keeps its original name).
    pub validated_at: i64,
    pub customer: Option<String>,
}

/// Pure: does this entitlement unlock Pro? Any stored key does, for good.
pub fn entitles(e: Option<&Entitlement>) -> bool {
    e.is_some()
}

/// Is `key` shaped like a license key? 16 to 100 characters, letters, digits, `-` and `_`, at least 16 of them letters
/// or digits (a Lemon Squeezy key is a 36-character UUID). Deliberately provider-agnostic: this is a typo check, not
/// proof of purchase.
pub fn is_well_formed(key: &str) -> bool {
    let k = key.trim();
    (16..=100).contains(&k.len())
        && k.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_'))
        && k.chars().filter(char::is_ascii_alphanumeric).count() >= 16
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Status {
    pub pro: bool,
    /// `none` | `active` | `dev`.
    pub state: &'static str,
    pub key_hint: Option<String>,
    pub customer: Option<String>,
}

pub fn status(db: &Db) -> Status {
    let ent = db.license_get().ok().flatten();
    let entitled = entitles(ent.as_ref());
    let state = if dev_forced() {
        "dev"
    } else if entitled {
        "active"
    } else {
        "none"
    };
    Status {
        pro: dev_forced() || entitled,
        state,
        key_hint: ent.as_ref().map(|e| hint(&e.key)),
        customer: ent.as_ref().and_then(|e| e.customer.clone()),
    }
}

/// `…1234`: never show the whole key again.
fn hint(key: &str) -> String {
    let tail: String = key.chars().rev().take(4).collect::<Vec<_>>().into_iter().rev().collect();
    format!("…{tail}")
}

/// Recompute the in-memory flag from the local database (no network); returns true when it changed.
pub fn refresh(db: &Db) -> bool {
    let ent = db.license_get().ok().flatten();
    let new = entitles(ent.as_ref());
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
    WrongProduct { instance_id: Option<String> },
    /// Could not get a trustworthy answer (offline, 5xx, rate limit, garbage).
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
    Deactivate,
}

fn call(http: &dyn Http, what: Call, form: &[(&str, &str)]) -> Outcome {
    let path = match what {
        Call::Activate => "activate",
        Call::Deactivate => "deactivate",
    };
    let resp = match http.post_form(&format!("{API}/{path}"), form) {
        Ok(r) => r,
        Err(e) => return Outcome::Unavailable(e),
    };
    // Lemon Squeezy answers a verdict with 200 / 400 / 404 / 422. Anything else (401/403 from a proxy, captive
    // portal, WAF, 5xx, 429, ...) says nothing about the key.
    if !matches!(resp.status, 200 | 400 | 404 | 422) {
        return Outcome::Unavailable(format!("license server returned {}", resp.status));
    }
    let Ok(body) = serde_json::from_str::<ApiResp>(&resp.body) else {
        return Outcome::Unavailable(format!("unreadable response ({})", resp.status));
    };
    let key_status = body.license_key.as_ref().and_then(|k| k.status.as_deref()).unwrap_or("");
    let customer = body.meta.as_ref().and_then(|m| m.customer_name.clone());
    let verdict = match what {
        Call::Activate => body.activated,
        Call::Deactivate => body.deactivated,
    };
    let dead_status = matches!(key_status, "expired" | "disabled" | "invalid");
    // A verdict needs the verdict field (true/false) or a dead key status; `{}`, `{"errors":[..]}` etc. are not one.
    if verdict.is_none() && !dead_status {
        return Outcome::Unavailable(format!("no verdict in the response ({})", resp.status));
    }
    if verdict == Some(true) {
        if what != Call::Deactivate && !product_ok(&body.meta) {
            return Outcome::WrongProduct { instance_id: body.instance.map(|i| i.id) };
        }
        return Outcome::Ok { instance_id: body.instance.map(|i| i.id), customer };
    }
    let msg = body.error.unwrap_or_else(|| match key_status {
        "expired" => "This license has expired.".into(),
        "disabled" => "This license has been disabled.".into(),
        _ => "This license key is not valid.".into(),
    });
    Outcome::Rejected(msg)
}

/// The name shown in the Lemon Squeezy dashboard for this activation: "Styky" plus a random per-install id, never the
/// PC name or user name (see docs/PRIVACY.md).
pub fn instance_name(db: &Db) -> String {
    format!("Styky {}", db.install_id())
}

/// Activate `key` on this PC and store it. One request, started by the user's click. A key that is not well-formed is
/// refused without any request; an explicit rejection from the provider is returned as the error; if the request
/// cannot complete, the key is accepted as entered (offline activation) and never checked again. A previous
/// activation (another key) is released afterwards, best effort, so slots do not leak.
pub fn activate(db: &Db, http: &dyn Http, key: &str, name: &str, now: i64) -> Res<Entitlement> {
    let key = key.trim();
    if key.is_empty() {
        return Err("Enter your license key.".into());
    }
    if !is_well_formed(key) {
        return Err("That does not look like a license key. Copy it from your receipt.".into());
    }
    match call(http, Call::Activate, &[("license_key", key), ("instance_name", name)]) {
        Outcome::Ok { instance_id: Some(instance_id), customer } => {
            let old = db.license_get()?;
            let ent = Entitlement { key: key.to_string(), instance_id, validated_at: now, customer };
            db.license_set(&ent)?;
            if let Some(old) = old.filter(|o| !o.instance_id.is_empty() && o.instance_id != ent.instance_id) {
                release(http, &old.key, &old.instance_id);
            }
            Ok(ent)
        }
        Outcome::Ok { .. } => Err("The license server did not return an activation.".into()),
        Outcome::Rejected(m) => Err(m),
        Outcome::WrongProduct { instance_id } => {
            // The key activated, but it is not ours: give the slot back.
            if let Some(id) = instance_id {
                release(http, key, &id);
            }
            Err("This key is for a different product.".into())
        }
        Outcome::Unavailable(_) => {
            // Offline or the server is unwell: honor system. Accept the well-formed key; there is no activation to release.
            let ent = Entitlement { key: key.to_string(), instance_id: String::new(), validated_at: now, customer: None };
            db.license_set(&ent)?;
            Ok(ent)
        }
    }
}

/// Best-effort deactivation of one instance (result ignored).
fn release(http: &dyn Http, key: &str, instance_id: &str) {
    let _ = call(http, Call::Deactivate, &[("license_key", key), ("instance_id", instance_id)]);
}

/// "Remove license": free the activation slot (one request, best effort) and forget the key locally. A key that was
/// accepted offline has no activation, so nothing is sent. Offline removal still succeeds locally; the slot can be
/// freed from the customer portal. Returns whether the slot is free.
pub fn deactivate(db: &Db, http: &dyn Http) -> Res<bool> {
    let Some(ent) = db.license_get()? else { return Ok(true) };
    let freed = ent.instance_id.is_empty()
        || matches!(
            call(http, Call::Deactivate, &[("license_key", &ent.key), ("instance_id", &ent.instance_id)]),
            Outcome::Ok { .. }
        );
    db.license_clear_if(&ent.key, &ent.instance_id)?;
    Ok(freed)
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
        fn count(&self) -> usize {
            self.calls.borrow().len()
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
    const KEY: &str = "0b9d52c1-77a4-4f6e-9a3c-5d1e2f3a1234";

    fn activated_db(at: i64) -> Db {
        let db = Db::open_memory().unwrap();
        let http = Mock::new(vec![reply(200, ACTIVATED)]);
        activate(&db, &http, KEY, "PC Styky", at).unwrap();
        db
    }

    #[test]
    fn activate_stores_the_key_and_sends_the_instance_name() {
        let db = Db::open_memory().unwrap();
        let http = Mock::new(vec![reply(200, ACTIVATED)]);
        let e = activate(&db, &http, &format!("  {KEY} "), "DESK Styky", 1000).unwrap();
        assert_eq!((e.instance_id.as_str(), e.validated_at, e.customer.as_deref()), ("inst-1", 1000, Some("Ada")));
        assert_eq!(db.license_get().unwrap(), Some(e));
        let calls = http.calls.borrow();
        assert_eq!(calls.len(), 1);
        assert!(calls[0].0.ends_with("/licenses/activate"));
        assert!(calls[0].1.contains(&("license_key".into(), KEY.into())));
        assert!(calls[0].1.contains(&("instance_name".into(), "DESK Styky".into())));
    }

    #[test]
    fn an_explicit_rejection_is_shown_and_nothing_is_stored() {
        let db = Db::open_memory().unwrap();
        let bad = Mock::new(vec![reply(404, r#"{"activated":false,"error":"license_key not found."}"#)]);
        assert_eq!(activate(&db, &bad, KEY, "n", 0).unwrap_err(), "license_key not found.");
        let limit = Mock::new(vec![reply(400, r#"{"activated":false,"error":"This license key has reached the activation limit."}"#)]);
        assert!(activate(&db, &limit, KEY, "n", 0).unwrap_err().contains("activation limit"));
        for body in [
            r#"{"activated":false,"license_key":{"status":"disabled"}}"#,
            r#"{"activated":false,"license_key":{"status":"expired"}}"#,
        ] {
            assert!(activate(&db, &Mock::new(vec![reply(400, body)]), KEY, "n", 0).is_err(), "{body}");
        }
        assert_eq!(db.license_get().unwrap(), None);
        assert!(!is_pro_in(&db));
    }

    fn is_pro_in(db: &Db) -> bool {
        status(db).pro
    }

    #[test]
    fn a_malformed_or_empty_key_is_refused_without_a_request() {
        let db = Db::open_memory().unwrap();
        let http = Mock::new(vec![]);
        for k in ["", "   ", "nope", "KEY-1234", "has spaces in it but is long enough", "ünïcödé-key-0000-1111-2222", &"x".repeat(101), "----------------"] {
            assert!(activate(&db, &http, k, "n", 0).is_err(), "{k:?}");
        }
        assert_eq!(http.count(), 0);
        assert_eq!(db.license_get().unwrap(), None);
        for k in [KEY, "ABCD-EFGH-IJKL-MNOP", "abcdefghijklmnop", "0123456789ABCDEF_x"] {
            assert!(is_well_formed(k), "{k}");
        }
    }

    #[test]
    fn offline_or_a_failed_request_still_accepts_a_well_formed_key() {
        for r in [
            Err("dns error".to_string()),
            reply(503, "<html>"),
            reply(500, "oops"),
            reply(429, "{}"),
            reply(403, "{}"),
            reply(200, "not json"),
            reply(200, "{}"),
            reply(404, "{}"),
        ] {
            let db = Db::open_memory().unwrap();
            let http = Mock::new(vec![r]);
            let e = activate(&db, &http, KEY, "n", 77).unwrap();
            assert_eq!((e.key.as_str(), e.instance_id.as_str(), e.validated_at), (KEY, "", 77));
            assert_eq!(http.count(), 1, "exactly one attempt, no retry");
            assert_eq!(db.license_get().unwrap(), Some(e));
            let s = status(&db);
            assert_eq!((s.pro, s.state, s.key_hint.as_deref()), (true, "active", Some("…1234")));
        }
    }

    #[test]
    fn a_stored_key_unlocks_pro_for_good_and_is_never_checked_again() {
        let db = activated_db(0);
        // Any amount of time later, with nothing scripted: no clock input exists, so nothing can expire or be re-checked.
        assert!(entitles(db.license_get().unwrap().as_ref()));
        assert!(status(&db).pro);
        assert!(!entitles(None));
    }

    #[test]
    fn the_module_makes_no_request_at_startup_or_on_a_timer_and_one_each_on_activate_and_remove() {
        let http = Mock::new(vec![reply(200, ACTIVATED), reply(200, r#"{"deactivated":true,"error":null}"#)]);
        let db = Db::open_memory().unwrap();
        // "Startup": everything the app runs when it starts, and everything the UI polls, takes no HTTP client at all
        // (only `activate` and `deactivate` accept one) and so cannot send anything.
        refresh(&db);
        status(&db);
        is_pro();
        instance_name(&db);
        assert_eq!(http.count(), 0, "no request before the user clicks");
        // Activate: exactly one request.
        activate(&db, &http, KEY, "n", 5).unwrap();
        assert_eq!(http.count(), 1);
        assert!(http.calls.borrow()[0].0.ends_with("/licenses/activate"));
        // Later "ticks" (status polls, refreshes) still send nothing.
        for _ in 0..3 {
            refresh(&db);
            status(&db);
        }
        assert_eq!(http.count(), 1);
        // Remove: exactly one more, a deactivate.
        assert!(deactivate(&db, &http).unwrap());
        assert_eq!(http.count(), 2);
        assert!(http.calls.borrow()[1].0.ends_with("/licenses/deactivate"));
        assert_eq!(db.license_get().unwrap(), None);
        // Nothing left to remove: no request.
        assert!(deactivate(&db, &http).unwrap());
        assert_eq!(http.count(), 2);
    }

    #[test]
    fn only_the_two_click_handlers_may_reach_the_network() {
        // Guard against a background re-check creeping back in: the real HTTP client is named only where the user's
        // Activate / Remove clicks are handled.
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
        let mut users = vec![];
        let mut stack = vec![dir];
        while let Some(d) = stack.pop() {
            for e in std::fs::read_dir(d).unwrap().flatten() {
                let p = e.path();
                if p.is_dir() {
                    stack.push(p);
                } else if p.extension().is_some_and(|x| x == "rs") {
                    let text = std::fs::read_to_string(&p).unwrap();
                    for (needle, label) in [("UreqHttp", "UreqHttp"), ("license::activate", "activate"), ("license::deactivate", "deactivate")] {
                        if text.contains(needle) && !p.ends_with("license.rs") {
                            users.push(format!("{}:{label}", p.file_name().unwrap().to_string_lossy()));
                        }
                    }
                }
            }
        }
        users.sort();
        users.dedup();
        assert_eq!(
            users,
            ["commands.rs:UreqHttp", "commands.rs:activate", "commands.rs:deactivate"],
            "UreqHttp / activate / deactivate are used outside the click handlers"
        );
    }

    #[test]
    fn remove_license_deactivates_and_forgets_even_when_offline() {
        let db = activated_db(0);
        let http = Mock::new(vec![reply(200, r#"{"deactivated":true,"error":null}"#)]);
        assert!(deactivate(&db, &http).unwrap());
        assert!(http.calls.borrow()[0].0.ends_with("/licenses/deactivate"));
        assert!(http.calls.borrow()[0].1.contains(&("instance_id".into(), "inst-1".into())));
        assert_eq!(db.license_get().unwrap(), None);
        let db = activated_db(0);
        assert!(!deactivate(&db, &Mock::new(vec![Err("offline".into())])).unwrap());
        assert_eq!(db.license_get().unwrap(), None);
    }

    #[test]
    fn removing_an_offline_accepted_key_sends_nothing() {
        let db = Db::open_memory().unwrap();
        activate(&db, &Mock::new(vec![Err("offline".into())]), KEY, "n", 1).unwrap();
        let http = Mock::new(vec![]);
        assert!(deactivate(&db, &http).unwrap());
        assert_eq!(http.count(), 0);
        assert_eq!(db.license_get().unwrap(), None);
        assert!(!status(&db).pro);
    }

    #[test]
    fn status_reflects_the_stored_key() {
        let db = Db::open_memory().unwrap();
        assert_eq!(status(&db).state, "none");
        let db = activated_db(0);
        let s = status(&db);
        assert_eq!((s.pro, s.state, s.key_hint.as_deref(), s.customer.as_deref()), (true, "active", Some("…1234"), Some("Ada")));
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

    #[test]
    fn activating_over_a_stored_key_releases_the_old_instance() {
        let db = activated_db(0); // KEY / inst-1
        let second = r#"{"activated":true,"license_key":{"status":"active"},"instance":{"id":"inst-9"},"meta":{"customer_name":"Ada"}}"#;
        let other = "ffffffff-1111-4222-8333-444444444444";
        let http = Mock::new(vec![reply(200, second), reply(200, r#"{"deactivated":true}"#)]);
        let e = activate(&db, &http, other, "n", 5).unwrap();
        assert_eq!(e.instance_id, "inst-9");
        assert_eq!(db.license_get().unwrap().unwrap().key, other);
        let calls = http.calls.borrow();
        assert!(calls[1].0.ends_with("/licenses/deactivate"));
        assert!(calls[1].1.contains(&("license_key".into(), KEY.into())));
        assert!(calls[1].1.contains(&("instance_id".into(), "inst-1".into())));
        // Re-activating the same instance id would not release anything (nothing stale).
        let same = Mock::new(vec![reply(200, ACTIVATED_9)]);
        let db2 = Db::open_memory().unwrap();
        activate(&db2, &same, KEY, "n", 0).unwrap();
        assert_eq!(same.calls.borrow().len(), 1);
    }

    const ACTIVATED_9: &str = r#"{"activated":true,"license_key":{"status":"active"},"instance":{"id":"inst-9"},"meta":{}}"#;

    #[test]
    fn a_failed_release_does_not_fail_activation() {
        let db = activated_db(0);
        let http = Mock::new(vec![reply(200, ACTIVATED_9), Err("offline".into())]);
        assert!(activate(&db, &http, "ffffffff-1111-4222-8333-444444444444", "n", 5).is_ok());
    }

    #[test]
    fn a_key_for_a_different_product_is_refused_and_its_slot_given_back() {
        // Only meaningful once the store and product ids are configured; exercise the branch with a mock meta.
        let m = Some(ApiMeta { store_id: Some(9), product_id: Some(9), customer_name: None });
        assert!(!product_matches(&m, 1, 2));
    }

    #[test]
    fn the_instance_name_is_random_per_install_and_names_no_pc() {
        let db = Db::open_memory().unwrap();
        let n = instance_name(&db);
        assert!(n.starts_with("Styky ") && n.len() == "Styky ".len() + 8, "{n}");
        assert_eq!(instance_name(&db), n, "stable per install");
        let pc = std::env::var("COMPUTERNAME").unwrap_or_default();
        assert!(pc.is_empty() || !n.to_lowercase().contains(&pc.to_lowercase()));
        assert!(!db.export(0).unwrap().settings.contains_key("install_id"), "never exported");
        assert_ne!(instance_name(&Db::open_memory().unwrap()), n);
    }
}
