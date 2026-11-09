import {
  AppWindow,
  BellRing,
  Check,
  Code2,
  FileText,
  Keyboard,
  Lock,
  Plus,
  Repeat,
  Rocket,
  Zap,
} from "lucide-react";
import { config, NETWORK_STATEMENT } from "../config";
import { Reveal, SectionHead } from "./util";

export function Problem() {
  const steps = [
    {
      icon: Zap,
      title: "Set it from anywhere",
      text: "Ctrl+Alt+N from any app, styky add from a terminal or a git hook, or ask Claude. One grammar for all of them: Deploy api fri 5pm #infra !!",
    },
    {
      icon: BellRing,
      title: "Notifies on time",
      text: "Native Windows toasts with Done and Snooze, a Snooze dropdown and a missed-while-away summary when the PC was off or asleep.",
    },
    {
      icon: Lock,
      title: "Local, with no account",
      text: "A SQLite file on your PC and a named pipe only your Windows user can open. MIT licensed, so you can read every line.",
    },
  ];
  return (
    <section className="x-section" id="why">
      <div className="x-wrap x-split">
        <Reveal>
          <p className="x-eyebrow">Why Styky</p>
          <p className="x-quote">
            Deploy on Friday. Rotate the keys. Check CI after the push. Small, single things that live only in your
            head <s>until it&rsquo;s too late.</s>
          </p>
          <p className="x-lead" style={{ fontSize: 16 }}>
            Not a project, not a ticket. Styky is where your tools and your assistant can put them, and where you can
            see them.
          </p>
        </Reveal>
        <div className="x-steps">
          {steps.map((s, i) => (
            <Reveal key={s.title} delay={i * 80}>
              <div className="x-card x-step">
                <div className="x-ico">
                  <s.icon size={20} strokeWidth={1.75} />
                </div>
                <div>
                  <h3>{s.title}</h3>
                  <p>{s.text}</p>
                </div>
              </div>
            </Reveal>
          ))}
        </div>
      </div>
    </section>
  );
}

const FEATURES: { icon: typeof Zap; title: string; text: string; pro?: boolean }[] = [
  { icon: Plus, title: "Global quick add", text: "A Spotlight-style bar with natural-language dates, #tags, @branch, !! priority, repeats and links. One shortcut, from any app." },
  { icon: BellRing, title: "Native notifications", text: "Windows toasts with a Snooze dropdown (10 min, 1 hour, this evening, tomorrow, custom) and Done." },
  { icon: Zap, title: "Priorities", text: "Low, normal, urgent and critical, from the editor, quick-add (!!, !!!), the CLI or MCP." },
  { icon: Code2, title: "Built for developers", text: "A branch chip (@release/2.3) that copies on click, a link field, project tags, and a CLI and MCP server." },
  { icon: Repeat, title: "Repeats that behave", text: "Daily, weekdays, weekly, monthly, with the 31st clamped to shorter months. Pro adds every 2 weeks, Mon/Wed/Fri, the last Friday of the month, end after N times." },
  { icon: FileText, title: "Markdown details", text: "Notes, code, links and checklists you can tick, right in the widget." },
  { icon: AppWindow, title: "A desktop panel", text: "A resizable Acrylic glass panel on the desktop layer that follows light and dark mode and your accent color. Pro adds tint, density and a paper skin." },
  { icon: Keyboard, title: "Keyboard-first", text: "Navigate, complete, snooze, edit and delete without touching the mouse." },
  { icon: Rocket, title: "Starts with Windows", text: "Launches at login by default and updates itself from signed GitHub releases. Everything stays on your PC, with JSON export and import." },
];

export function Features() {
  return (
    <section className="x-section" id="features">
      <div className="x-wrap">
        <SectionHead
          eyebrow="Features"
          title="Small on screen. Complete underneath."
          lead="Free and open source, except the items marked Pro."
          center
        />
        <div className="x-grid c3">
          {FEATURES.map((f, i) => (
            <Reveal key={f.title} delay={(i % 3) * 70}>
              <div className="x-card" style={{ height: "100%" }}>
                <div className="x-ico">
                  <f.icon size={20} strokeWidth={1.75} />
                </div>
                <h3>
                  {f.title} {f.pro ? <span className="x-badge">Pro</span> : null}
                </h3>
                <p>{f.text}</p>
              </div>
            </Reveal>
          ))}
        </div>
      </div>
    </section>
  );
}

const KEYS: { keys: string[]; label: string }[] = [
  { keys: ["↑", "↓"], label: "Move through reminders" },
  { keys: ["Space"], label: "Complete (or restore)" },
  { keys: ["Enter"], label: "Expand or collapse" },
  { keys: ["E"], label: "Edit" },
  { keys: ["S"], label: "Snooze" },
  { keys: ["Del"], label: "Delete, with Undo" },
  { keys: ["Ctrl", "N"], label: "New reminder" },
  { keys: ["Ctrl", "Enter"], label: "Save and close" },
  { keys: ["Esc"], label: "Collapse / cancel" },
];

export function KeyboardSection() {
  return (
    <section className="x-section alt" id="keyboard">
      <div className="x-wrap">
        <SectionHead
          eyebrow="Keyboard-first"
          title="Your hands never leave the keyboard."
          lead="Click the demo at the top of the page, then try these keys."
          center
        />
        <Reveal>
          <div className="x-keys">
            <div className="x-key hot">
              <span>
                <kbd>Ctrl</kbd>
                <kbd>Alt</kbd>
                <kbd>N</kbd>
              </span>
              <span>Quick add from any app. Rebind it in Settings.</span>
            </div>
            {KEYS.map((k) => (
              <div className="x-key" key={k.label}>
                <span>
                  {k.keys.map((x) => (
                    <kbd key={x}>{x}</kbd>
                  ))}
                </span>
                <span>{k.label}</span>
              </div>
            ))}
          </div>
        </Reveal>
      </div>
    </section>
  );
}

export function Privacy() {
  const points = [
    ["No account, no cloud.", "There is no sign-up and no server to sync to."],
    ["No telemetry.", "No analytics, ads or crash reporting. Styky never learns how you use it."],
    ["Your data is a file.", "Reminders live in a local SQLite database. Export or import JSON any time."],
    ["A private local API.", "The CLI and MCP server use a named pipe restricted to your Windows account. Nothing listens on the network."],
    ["Your assistant, your call.", "If you connect Claude or another client, what it reads from Styky goes to that assistant."],
    ["Open source.", "Read every line on GitHub, or build it yourself."],
  ];
  return (
    <section className="x-section" id="privacy">
      <div className="x-wrap x-priv">
        <div>
          <Reveal>
            <p className="x-eyebrow">Privacy &amp; local-first</p>
            <h2 className="x-h2">Your reminders never leave your PC.</h2>
            <p className="x-lead">
              Styky works fully offline. {NETWORK_STATEMENT}
            </p>
          </Reveal>
          <Reveal delay={80}>
            <ul className="x-ticks">
              {points.map(([b, t]) => (
                <li key={b}>
                  <Check size={20} strokeWidth={2.25} />
                  <span>
                    <b>{b}</b> {t}
                  </span>
                </li>
              ))}
            </ul>
          </Reveal>
        </div>
        <Reveal delay={120}>
          <div className="x-card x-file">
            <b>%APPDATA%\com.styky.app\</b>
            <br />
            &nbsp;&nbsp;styky.db
            <br />
            &nbsp;&nbsp;.window-state.json
            <br />
            <span className="x-pill">All of it, on your disk</span>
            <p style={{ marginTop: 18, fontFamily: "var(--font)", fontSize: 14, color: "var(--label-2)" }}>
              Read the full <a href={`${import.meta.env.BASE_URL}privacy`}>privacy policy</a>. It is short.
            </p>
          </div>
        </Reveal>
      </div>
    </section>
  );
}

const FAQ: { q: string; a: React.ReactNode }[] = [
  {
    q: "Windows shows a SmartScreen warning when I run the installer. Is it safe?",
    a: (
      <>
        <p>
          Styky is a new, independent app and the installer is not code-signed yet, so Windows SmartScreen shows
          &ldquo;Windows protected your PC&rdquo; for files it hasn&rsquo;t seen often. Click <b>More info</b>, then{" "}
          <b>Run anyway</b>.
        </p>
        <p>
          The whole app is open source, so you can read the code on <a href={config.repoUrl}>GitHub</a> or build the
          installer yourself.
        </p>
      </>
    ),
  },
  {
    q: "If it's open source, why would I pay for Pro?",
    a: (
      <>
        <p>
          Styky Pro runs on the honor system, not DRM. The Pro code lives in the public repo behind a license check,
          and anyone can build it themselves. You pay for the convenience of a signed, auto-updating build and to keep
          one developer working on Styky.
        </p>
        <p>Nothing that is free today becomes paid.</p>
      </>
    ),
  },
  {
    q: "What works without Pro?",
    a: (
      <p>
        Everything in the panel, every priority including Critical, the whole <code>styky</code> CLI and all seven MCP
        tools. Pro adds advanced repeats, nagging, early warnings and themes.
      </p>
    ),
  },
  {
    q: "Does the Styky app have to be running for the CLI?",
    a: (
      <p>
        Yes. <code>styky</code> talks to the running app, so with Styky closed you get &ldquo;Styky isn&rsquo;t
        running&rdquo; and exit code 2. Styky starts with Windows by default.
      </p>
    ),
  },
  {
    q: "Does it work with ChatGPT?",
    a: (
      <p>
        Not directly. ChatGPT only connects to remote HTTPS MCP servers, and <code>styky mcp</code> is a local one.
        Claude Desktop, Claude Code and other clients that launch a local command work today.
      </p>
    ),
  },
  {
    q: "How many devices does a license cover?",
    a: <p>About three of your own PCs. Activation needs the internet once; after that Pro works offline.</p>,
  },
  {
    q: "Which versions of Windows are supported?",
    a: (
      <p>
        Windows 11 is the target and looks best (the glass panel uses Acrylic). Windows 10 version 1809 or newer also
        works. 64-bit only, and the WebView2 runtime is already part of Windows 11.
      </p>
    ),
  },
  {
    q: "Does it need an account or an internet connection?",
    a: <p>No. Styky works fully offline and never asks you to sign in. (Activating a Pro key needs a connection once.)</p>,
  },
  {
    q: "Is there a Microsoft Store version?",
    a: (
      <p>
        A Store listing is in review. The CLI and MCP server ship with the installer on GitHub Releases; the Store
        build does not include them yet.
      </p>
    ),
  },
];

export function Faq() {
  return (
    <section className="x-section" id="faq">
      <div className="x-wrap">
        <SectionHead eyebrow="FAQ" title="Questions, answered." center />
        <Reveal>
          <div className="x-faq">
            {FAQ.map((f) => (
              <details key={f.q}>
                <summary>
                  {f.q}
                  <Plus size={18} strokeWidth={2} aria-hidden />
                </summary>
                <div className="x-ans">{f.a}</div>
              </details>
            ))}
          </div>
        </Reveal>
      </div>
    </section>
  );
}
