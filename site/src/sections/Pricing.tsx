import { useEffect } from "react";
import { Check, Download, Hourglass, ShieldCheck } from "lucide-react";
import { checkoutReady, config, money } from "../config";
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
        Pre-order for {money(config.priceEarly)}
      </a>
      <p className="x-buy-hint">Secure checkout by Lemon Squeezy. One-time payment, no subscription.</p>
    </>
  );
}

const FREE = [
  "The glass desktop panel, light and dark",
  "Natural-language quick add, from anywhere",
  "Native notifications with Snooze and Done",
  "Daily, weekday, weekly and monthly repeats",
  "Markdown notes and tickable checklists",
  "Tags, branch chips and links",
  "JSON export and import",
  "Unlimited reminders",
  "Basic CLI and AI capture (add, list), arriving with Pro v1",
];

const PRO: { t: string; d: string }[] = [
  {
    t: "Local API, CLI and MCP server",
    d: "styky add, ls, done, snooze, update and delete, plus bulk actions and filters. Connect Claude, ChatGPT or any MCP client.",
  },
  {
    t: "Escalating reminders and pre-alerts",
    d: "Heads-up 5 minutes to 1 day before. Urgent items re-notify until you are done, with an optional sound. A new Critical priority.",
  },
  {
    t: "Themes",
    d: "Panel tint and transparency, comfortable or compact density, accent override, a sticky-note paper skin, font size.",
  },
  {
    t: "Advanced repeats",
    d: "Every 2 weeks, Mon/Wed/Fri, the last Friday of the month, end after N times or on a date.",
  },
  { t: "All future Pro features", d: "One key covers about three of your own PCs. Works offline after activation." },
];

export function Pricing() {
  const { url } = useRelease();
  return (
    <section className="x-section" id="pricing">
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
                Styky Pro <span className="x-badge">Early-bird pre-order</span>
              </div>
              <div className="x-amount">
                <strong>{money(config.priceEarly)}</strong>
                <span>
                  one-time &middot; becomes {money(config.priceRegular)} when Pro v1 ships
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
                  <b>Ships in an upcoming update.</b> Pro v1 is being built now. Pre-order at the early-bird price and
                  your license key unlocks it the day it ships.
                </span>
              </div>
              <div className="x-note" style={{ background: "color-mix(in srgb, var(--green) 12%, transparent)" }}>
                <ShieldCheck size={18} strokeWidth={2} style={{ color: "var(--green)" }} />
                <span>
                  <b>Full refund if Pro doesn&rsquo;t ship.</b> No questions asked.
                </span>
              </div>
              <BuyButton />
            </div>
          </Reveal>
        </div>

        <Reveal delay={60}>
          <div className="x-card x-chat">
            <div>
              <p className="x-eyebrow">Pro &middot; Claude &amp; ChatGPT</p>
              <h3 style={{ fontSize: 26, lineHeight: 1.2, letterSpacing: "-0.025em", fontWeight: 650 }}>
                &ldquo;Remind me to rotate the keys on Friday.&rdquo;
              </h3>
              <p style={{ marginTop: 12, fontSize: 16, color: "var(--label-2)" }}>
                Pro ships a small local API, a <code>styky</code> command line and an MCP server, so the assistant
                you already use can add and manage your reminders. It all stays on your machine.
              </p>
              <div className="x-term" aria-label="Example commands">
                <i>$ </i>styky add &quot;Rotate keys fri 10am #infra !!&quot;{"\n"}
                <i>$ </i>styky ls --due today --json{"\n"}
                <i>$ </i>styky done 4
              </div>
            </div>
            <div className="x-chat-box" role="img" aria-label="Illustration: asking an AI assistant to add a Styky reminder">
              <div className="x-bubble me">Remind me to rotate the keys on Friday at 10, it&rsquo;s urgent.</div>
              <div className="x-tool">styky.add &#123; title: &quot;Rotate the keys&quot;, due: &quot;fri 10:00&quot;, priority: urgent &#125;</div>
              <div className="x-bubble ai">Added &ldquo;Rotate the keys&rdquo; for Friday at 10:00, marked urgent.</div>
              <p className="x-caption-sm">Illustration of the planned MCP connector. Arrives with Pro v1.</p>
            </div>
          </div>
        </Reveal>
      </div>
    </section>
  );
}
