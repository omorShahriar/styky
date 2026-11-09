import { useEffect, useState } from "react";
import { Moon, Sun } from "lucide-react";
import { ANALYTICS_SENTENCE, config, NETWORK_STATEMENT } from "../config";

const BASE = import.meta.env.BASE_URL;

/** lucide dropped brand icons; this is the GitHub mark. */
export function Github({ size = 18 }: { size?: number; strokeWidth?: number }) {
  return (
    <svg width={size} height={size} viewBox="0 0 16 16" fill="currentColor" aria-hidden>
      <path d="M8 0C3.58 0 0 3.58 0 8c0 3.54 2.29 6.53 5.47 7.59.4.07.55-.17.55-.38 0-.19-.01-.82-.01-1.49-2.01.37-2.53-.49-2.69-.94-.09-.23-.48-.94-.82-1.13-.28-.15-.68-.52-.01-.53.63-.01 1.08.58 1.23.82.72 1.21 1.87.87 2.33.66.07-.52.28-.87.51-1.07-1.78-.2-3.64-.89-3.64-3.95 0-.87.31-1.59.82-2.15-.08-.2-.36-1.02.08-2.12 0 0 .67-.21 2.2.82A7.6 7.6 0 0 1 8 4.58c.68 0 1.36.09 2 .27 1.53-1.04 2.2-.82 2.2-.82.44 1.1.16 1.92.08 2.12.51.56.82 1.27.82 2.15 0 3.07-1.87 3.75-3.65 3.95.29.25.54.73.54 1.48 0 1.07-.01 1.93-.01 2.2 0 .21.15.46.55.38A8.01 8.01 0 0 0 16 8c0-4.42-3.58-8-8-8Z" />
    </svg>
  );
}

/** Light/dark switch. `onChange` lets the homepage keep the live demo's mocked theme setting in step. */
export function ThemeToggle({ onChange }: { onChange?: (t: "light" | "dark") => void }) {
  // `document` does not exist while the legal pages are prerendered at build time.
  const [theme, setTheme] = useState(() =>
    typeof document === "undefined" ? "light" : (document.documentElement.dataset.theme ?? "light"),
  );
  useEffect(() => {
    document.documentElement.dataset.theme = theme;
  }, [theme]);
  const toggle = () => {
    const next = theme === "dark" ? "light" : "dark";
    setTheme(next);
    onChange?.(next);
    try {
      localStorage.setItem("styky-theme", next);
    } catch {
      /* private mode */
    }
  };
  return (
    <button className="x-icon-btn" onClick={toggle} aria-label={theme === "dark" ? "Switch to light theme" : "Switch to dark theme"}>
      {theme === "dark" ? <Sun size={18} strokeWidth={1.75} /> : <Moon size={18} strokeWidth={1.75} />}
    </button>
  );
}

/** Header of the legal pages: brand (home) and the same two icon buttons as the homepage nav. */
export function LegalNav() {
  return (
    <header className="x-nav">
      <div className="x-wrap x-nav-in">
        <a className="x-brand" href={BASE} aria-label="Styky, home">
          <img src={`${BASE}favicon.svg`} width={26} height={26} alt="" />
          Styky
        </a>
        <div className="x-nav-end">
          <ThemeToggle />
          <a className="x-icon-btn" href={config.repoUrl} aria-label="Styky on GitHub" rel="noopener">
            <Github size={18} strokeWidth={1.75} />
          </a>
        </div>
      </div>
    </header>
  );
}

export function Footer() {
  return (
    <footer className="x-footer">
      <div className="x-wrap">
        <div className="x-foot-in">
          <a className="x-brand" href={`${BASE}#top`} aria-label="Styky, back to top">
            <img src={`${BASE}favicon.svg`} width={24} height={24} alt="" />
            Styky
          </a>
          <nav className="x-foot-links" aria-label="Footer">
            <a href={config.repoUrl} rel="noopener">
              GitHub
            </a>
            <a href={config.releasesUrl} rel="noopener">
              Releases
            </a>
            <a href={config.licenseUrl} rel="noopener">
              MIT License
            </a>
            <a href={`${BASE}refund`}>Refund policy</a>
            <a href={`${BASE}terms`}>Terms</a>
            <a href={`${BASE}privacy`}>Privacy</a>
            <a href={`mailto:${config.contactEmail}`}>{config.contactEmail}</a>
          </nav>
        </div>
        <p className="x-fine">
          &copy; 2026 {config.author}. Styky is open source under the MIT License. Windows and Windows 11 are
          trademarks of Microsoft; Styky is not affiliated with or endorsed by Microsoft.
        </p>
        <p className="x-fine">
          {NETWORK_STATEMENT} {ANALYTICS_SENTENCE}
        </p>
      </div>
    </footer>
  );
}
