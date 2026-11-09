import { useState } from "react";
import { openUrl } from "@tauri-apps/plugin-opener";
import { api } from "../lib/api";
import { PRO_URL, useLicense } from "../lib/pro";

const BENEFITS = [
  "Advanced repeats: every 2 weeks, Mon/Wed/Fri, last Friday of the month, end after N",
  "Nagging: switch it on for a reminder and it notifies again until you act",
  "Early alerts: a heads-up before the due time",
  "Themes: tint, transparency, density, accent, a sticky-note skin",
];

/** Settings → Styky Pro: benefits, a link to the Pro page, key entry, activated state. Honor system: the free tier stays complete. */
export function ProCard() {
  const lic = useLicense();
  const [key, setKey] = useState("");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [note, setNote] = useState<string | null>(null);

  if (!lic) return null;
  const owned = lic.state === "active" || lic.state === "dev";

  const activate = async () => {
    setBusy(true);
    setError(null);
    try {
      await api.licenseActivate(key);
      setKey("");
    } catch (e) {
      setError(String(e));
    } finally {
      setBusy(false);
    }
  };

  const remove = async () => {
    setBusy(true);
    setNote(null);
    try {
      const freed = await api.licenseRemove();
      setNote(freed ? "License removed. The activation slot was freed." : "License removed from this PC. You were offline, so the activation slot is still used; free it from your receipt page.");
    } catch (e) {
      setError(String(e));
    } finally {
      setBusy(false);
    }
  };

  return (
    <section className="group" id="pro-card">
      <h2>Styky Pro</h2>
      <div className="card pro-card">
        {owned ? (
          <div className="pro-body">
            <div className="pro-head">
              <span className="pro-badge" aria-label="Supporter">
                <span aria-hidden>♥</span> Supporter
              </span>
              <span className="muted">
                {lic.state === "dev"
                  ? "Forced on for development (STYKY_PRO_DEV)"
                  : lic.customer
                    ? `Thank you, ${lic.customer}!`
                    : "Thank you for supporting Styky!"}
              </span>
            </div>
            {lic.keyHint && <p className="muted small">License {lic.keyHint}</p>}
            {lic.state !== "dev" && (
              <div className="pro-actions">
                <button className="btn" disabled={busy} onClick={() => void remove()}>
                  Remove license
                </button>
              </div>
            )}
          </div>
        ) : (
          <div className="pro-body">
            <p className="pro-lede">
              Styky is free and stays complete. Pro is a one-time unlock that adds power features and funds the project.
            </p>
            <ul className="pro-benefits">
              {BENEFITS.map((b) => (
                <li key={b}>{b}</li>
              ))}
            </ul>
            <div className="pro-actions">
              <button className="btn" onClick={() => void openUrl(PRO_URL).catch(() => {})}>
                Get Pro
              </button>
            </div>
            <form
              className="pro-key"
              onSubmit={(e) => {
                e.preventDefault();
                if (key.trim() && !busy) void activate();
              }}
            >
              <input
                type="text"
                value={key}
                spellCheck={false}
                autoComplete="off"
                placeholder="Paste your license key"
                aria-label="License key"
                onChange={(e) => setKey(e.target.value)}
              />
              <button className="btn" type="submit" disabled={busy || !key.trim()}>
                {busy ? "Checking…" : "Activate"}
              </button>
            </form>
            <p className="muted small">Styky checks the key once, online, when you click Activate. Offline, it accepts the key as entered.</p>
          </div>
        )}
        {error && (
          <p className="inline-error pro-msg" role="alert">
            {error}
          </p>
        )}
        {note && <p className="muted small pro-msg">{note}</p>}
      </div>
    </section>
  );
}
