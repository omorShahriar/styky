import { useState } from "react";
import { openUrl } from "@tauri-apps/plugin-opener";
import { api } from "../lib/api";
import { useLicense } from "../lib/pro";

const BENEFITS = [
  "Local API, the styky CLI and AI connectors (Claude, ChatGPT, any MCP client)",
  "Pre-alerts and Critical reminders that keep nagging until you act",
  "Themes: tint, transparency, density, accent, a sticky-note skin",
  "Advanced repeats: every 2 weeks, Mon/Wed/Fri, last Friday of the month, end after N",
];

/** Settings → Styky Pro: benefits, buy link, key entry, activated state. Honor system: the free tier stays complete. */
export function ProCard() {
  const lic = useLicense();
  const [key, setKey] = useState("");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [note, setNote] = useState<string | null>(null);

  if (!lic) return null;
  const owned = lic.state === "active" || lic.state === "grace-expired" || lic.state === "dev";

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
      setNote(freed ? "License removed. The activation slot was freed." : "License removed from this PC. You were offline, so the slot is still used; free it from your Lemon Squeezy receipt page.");
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
            {lic.state === "grace-expired" && (
              <p className="inline-error" role="status">
                Pro is paused: Styky hasn't been able to verify your license for over 30 days. Connect to the internet and it comes back on its own.
              </p>
            )}
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
              <button className="btn primary" onClick={() => void openUrl(lic.buyUrl)}>
                Buy Styky Pro
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
