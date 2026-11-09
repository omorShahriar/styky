import { lazy, Suspense } from "react";
import { Download, Star } from "lucide-react";
import { config } from "../config";
import { setSiteTheme } from "../mock/site-theme";
import { release } from "../release";
import { Github, ThemeToggle } from "./Shell";

/** The demo pulls in the whole app UI (motion, markdown, the date parser): load it after first paint. */
const Demo = lazy(() => import("../demo/Demo"));

const BASE = import.meta.env.BASE_URL;

export function Nav() {
  return (
    <header className="x-nav">
      <div className="x-wrap x-nav-in">
        <a className="x-brand" href="#top" aria-label="Styky, back to top">
          <img src={`${BASE}favicon.svg`} width={26} height={26} alt="" />
          Styky
        </a>
        <nav className="x-nav-links" aria-label="Sections">
          <a href="#connect">Claude &amp; CLI</a>
          <a href="#quick-add">Quick add</a>
          <a href="#features">Features</a>
          <a href="#privacy">Privacy</a>
          <a href="#pricing">Pro</a>
          <a href="#faq">FAQ</a>
        </nav>
        <div className="x-nav-end">
          <ThemeToggle onChange={setSiteTheme} />
          <a className="x-icon-btn" href={config.repoUrl} aria-label="Styky on GitHub" rel="noopener">
            <Github size={18} strokeWidth={1.75} />
          </a>
        </div>
      </div>
    </header>
  );
}

export function Hero() {
  const { version, stars } = release;
  return (
    <section className="x-hero" id="top">
      <div className="x-wrap">
        <div className="x-hero-copy">
          <h1 className="x-h1">
            Reminders you control from <em>Claude, your terminal and git hooks.</em>
          </h1>
          <p className="x-lead">
            Styky is a free, open-source Windows reminder widget with a local CLI and MCP server. Add from anywhere with
            Ctrl+Alt+N or <code>styky add</code>, and get a native notification with Done and Snooze when it is
            due.
          </p>
          <div className="x-cta">
            <a className="x-btn primary" href={config.downloadUrl} rel="noopener">
              <Download size={18} strokeWidth={2} /> Download for Windows
            </a>
            <a className="x-stars" href={config.repoUrl} rel="noopener" aria-label={`Star Styky on GitHub${stars != null ? `, ${stars} stars` : ""}`}>
              <Star size={16} strokeWidth={2} fill="currentColor" />
              {stars != null && stars >= 10 ? <span className="tnum">{stars.toLocaleString()}</span> : null}
              <span>{stars != null && stars >= 10 ? "stars on GitHub" : "Star on GitHub"}</span>
            </a>
            <a className="x-btn secondary" href="#connect">
              See the commands
            </a>
          </div>
          <pre className="x-term x-hero-term" aria-label="Example commands">
            <i>$ </i>styky add &quot;Deploy api fri 5pm #infra !!&quot;{"\n"}
            <i>$ </i>claude mcp add styky -- styky mcp
          </pre>
          <p className="x-meta">
            Free and open source (MIT) · v{version} · Windows 10 &amp; 11 · Per-user install, no admin prompt
          </p>
        </div>
        <Suspense fallback={<div className="x-demo-skeleton" aria-hidden />}>
          <Demo base={BASE} />
        </Suspense>
      </div>
    </section>
  );
}
