import { useEffect } from "react";
import { Check, Download, Hourglass, ShieldCheck } from "lucide-react";
import { checkoutReady, config, money, proVersion, versionAtLeast } from "../config";
import { Reveal, SectionHead, useRelease } from "./util";

declare global {
  interface Window {
    createLemonSqueezy?: () => void;
    LemonSqueezy?: unknown;
  }
}

const LEMON_JS = "https://assets.lemonsqueezy.com/lemon.js";

/** Lemon Squeezy overlay checkout. While the URL is still the placeholder it renders a disabled "Coming soon". */
function BuyButton() {
  useEffect(() => {
    if (!checkoutReady) return;
    const init = () => window.createLemonSqueezy?.();
    const existing = document.querySelector<HTMLScriptElement>(`script[src="${LEMON_JS}"]`);
    if (existing) return init();
    const s = document.createElement("script");
    s.src = LEMON_JS;
    s.defer = true;
    s.onload = init;
    document.head.appendChild(s);
  }, []);

  if (!checkoutReady) {
    return (
      <>
        <span className="x-btn" role="link" aria-disabled="true" tabIndex={0}>
          Coming soon
        </span>
        <p className="x-buy-hint">Checkout opens soon. Star the repo to hear when it does.</p>
      </>
    );
  }
  const href = `${config.checkoutUrl}${config.checkoutUrl.includes("?") ? "&" : "?"}embed=1`;
  return (
    <>
      <a className="x-btn primary lemonsqueezy-button" href={href}>
        Get Pro for {money(config.priceEarly)}
      </a>
      <p className="x-buy-hint">Secure checkout by Lemon Squeezy. One-time payment, no subscription.</p>
    </>
  );
}

const FREE = [
  "The glass desktop panel, light and dark",
  "Natural-language quick add, from anywhere",
  "Native notifications with Snooze and Done",
  "styky add and styky ls, and the MCP tools styky_add and styky_list",
  "Daily, weekday, weekly and monthly repeats",
  "Markdown notes, tickable checklists, tags, branch chips, links",
  "JSON export and import",
  "Unlimited reminders",
];

const PRO: { t: string; d: string }[] = [
  {
    t: "Full CLI and MCP control",
    d: "styky get, done, snooze, edit and rm, and the matching MCP tools, so Claude can complete and reschedule too.",
  },
  {
    t: "Critical reminders and pre-alerts",
    d: "!!! re-notifies every N minutes (default 10) until you act. Heads-up toasts 5 minutes to 1 day before.",
  },
  {
    t: "Advanced repeats",
    d: "Every 2 weeks, Mon/Wed/Fri, the last Friday of the month, end after N times or on a date.",
  },
  {
    t: "Themes",
    d: "Panel tint and transparency, comfortable or compact density, accent override, a paper skin, font size.",
  },
  { t: "All future Pro features", d: "One key covers about three of your own PCs. Works offline after activation." },
];

export function Pricing() {
  const { url, version } = useRelease();
  const shipped = versionAtLeast(version, proVersion);
  return (
    <section className="x-section alt" id="pricing">
      <div className="x-wrap">
        <SectionHead
          eyebrow="Pricing"
          title="Free, and complete. Pro is for power users."
          lead="Styky stays free and MIT-licensed. Pro is a one-time unlock that adds power features on top. Nothing that is free today becomes paid."
          center
        />
        <div className="x-price-grid">
          <Reveal>
            <div className="x-card x-plan free" style={{ height: "100%" }}>
              <div className="x-plan-name">Free</div>
              <div className="x-amount">
                <strong>$0</strong>
                <span>forever</span>
              </div>
              <p className="x-plan-sub">Everything you need to never miss a thing.</p>
              <ul>
                {FREE.map((f) => (
                  <li key={f}>
                    <Check size={18} strokeWidth={2.25} />
                    <span>{f}</span>
                  </li>
                ))}
              </ul>
              <a className="x-btn secondary" href={url} rel="noopener">
                <Download size={18} strokeWidth={2} /> Download for Windows
              </a>
            </div>
          </Reveal>
          <Reveal delay={80}>
            <div className="x-card x-plan pro" style={{ height: "100%" }}>
              <div className="x-plan-name">
                Styky Pro <span className="x-badge">Early-bird price</span>
              </div>
              <div className="x-amount">
                <strong>{money(config.priceEarly)}</strong>
                <span>
                  one-time &middot; becomes {money(config.priceRegular)} after the early-bird period
                </span>
              </div>
              <p className="x-plan-sub">Everything in Free, plus:</p>
              <ul>
                {PRO.map((f) => (
                  <li key={f.t}>
                    <Check size={18} strokeWidth={2.25} />
                    <span>
                      {f.t}
                      <small>{f.d}</small>
                    </span>
                  </li>
                ))}
              </ul>
              <div className="x-note">
                <Hourglass size={18} strokeWidth={2} />
                <span>
                  {shipped ? (
                    <>
                      <b>Included in v{proVersion} and later.</b> Buy a key, paste it in Settings, Styky Pro.
                    </>
                  ) : (
                    <>
                      <b>Arrives in v{proVersion}.</b> Buy now at the early-bird price and the key unlocks Pro when
                      v{proVersion} is out.
                    </>
                  )}
                </span>
              </div>
              <div className="x-note" style={{ background: "color-mix(in srgb, var(--green) 12%, transparent)" }}>
                <ShieldCheck size={18} strokeWidth={2} style={{ color: "var(--green)" }} />
                <span>
                  <b>Honor system, no DRM.</b> The Pro code is in the public repo. You pay for the signed build and to
                  keep it maintained.
                </span>
              </div>
              <BuyButton />
            </div>
          </Reveal>
        </div>

      </div>
    </section>
  );
}
