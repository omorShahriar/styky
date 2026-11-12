/**
 * Site configuration. Edit this file; see site/README.md.
 * Release facts (version, stars) are NOT here: they come from src/release.json, refreshed at build time
 * (scripts/release-data.mjs). The browser never calls a GitHub API.
 */

export const config = {
  repo: "omorShahriar/styky",
  repoUrl: "https://github.com/omorShahriar/styky",
  releasesUrl: "https://github.com/omorShahriar/styky/releases/latest",
  /** Static URL: every release also uploads the installer under this unversioned name, so it never needs a redeploy. */
  downloadUrl: "https://github.com/omorShahriar/styky/releases/latest/download/Styky_x64-setup.exe",
  licenseUrl: "https://github.com/omorShahriar/styky/blob/main/LICENSE",
  author: "Omor Shahriar",
  authorUrl: "https://github.com/omorShahriar",
  publisher: "Small Hinge Labs",
  publisherUrl: "https://smallhingelabs.com/",
  contactEmail: "hello@smallhingelabs.com",
  lemonBuyerTermsUrl: "https://www.lemonsqueezy.com/buyer-terms",
} as const;

/**
 * The one canonical network statement. Keep it byte-identical with docs/PRIVACY.md and the README.
 * (Add the GitHub-watch clause here, and there, only when GitHub watch ships.)
 */
export const NETWORK_STATEMENT =
  "No telemetry. Styky only goes online to check for updates and to activate or remove a Pro key when you click the button.";

/** Shown next to the statement on the site. */
export const ANALYTICS_SENTENCE = "This site counts visits with Cloudflare Web Analytics: no cookies, no personal data.";

/** Pro teaser copy (exactly these four strings, nothing else, until the /pro page ships). */
export const proTeaser = {
  heading: "Styky Pro",
  tagline: "Reminders that know when they're done.",
  price: "$12 one-time · coming Nov 30",
  line: "Styky is free. Pro is how it stays that way.",
} as const;

/** C8 flag, set in vite.config.ts (SHOW_AGENT_FEATURES_DEFAULT). False means no agent/MCP/CLI copy is in the build. */
declare const __SHOW_AGENT_FEATURES__: boolean;
export const SHOW_AGENT_FEATURES: boolean = __SHOW_AGENT_FEATURES__;

/** The approved plain description (also the meta description). Used where a CLI/MCP sentence has to be replaced. */
export const APPROVED_DESCRIPTION =
  "A small desktop reminder widget for Windows 11. It stays visible after Win+D and nudges you with notifications you can act on.";

/** The approved tagline. Replaces a headline that carried a CLI/MCP claim. */
export const TAGLINE = "Remember the small stuff that breaks big things.";

/** The release that introduced the CLI and the MCP server. */
export const proVersion = "0.2.0";

/** True when `version` ("0.2.1", "v0.3.0") is `min` or newer. Compares numeric parts only. */
export function versionAtLeast(version: string, min: string): boolean {
  const n = (v: string) => v.replace(/^v/, "").split(/[.-]/).slice(0, 3).map((x) => parseInt(x, 10) || 0);
  const [a, b] = [n(version), n(min)];
  for (let i = 0; i < 3; i++) if ((a[i] ?? 0) !== (b[i] ?? 0)) return (a[i] ?? 0) > (b[i] ?? 0);
  return true;
}

/** Date shown on the legal pages ("Last updated"). */
export const legalUpdated = "November 9, 2026";
