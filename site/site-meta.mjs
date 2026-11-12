// The single source for the site's head metadata: URLs, titles, descriptions, JSON-LD.
// Used by vite.config.ts (the htmlVars plugin fills the placeholders in the *.html entries) and by
// scripts/prerender.mjs (sitemap.xml, robots.txt). Plain .mjs so Node scripts and the Vite config can both import it.
//
// Rule C8: the agent copy (CLI, MCP server, Claude) is metadata too. It is only used when the build flag
// SHOW_AGENT_FEATURES is on (vite.config.ts); a flag-off build falls back to the plain title and description below and
// carries no agent wording anywhere.
import fs from "node:fs";

/** Public base path. "/" on Cloudflare Pages (custom domain); "/styky/" only for GitHub Pages. */
export const base = process.env.SITE_BASE || "/";
/** Absolute site URL (with trailing slash), used for canonical / OG tags, the sitemap and robots.txt. */
export const siteUrl = (process.env.SITE_URL || "https://styky.smallhingelabs.com/").replace(/\/?$/, "/");

export const siteName = "Styky";
export const publisher = { name: "Small Hinge Labs", url: "https://smallhingelabs.com" };

/** The approved description. scripts/prerender.mjs checks it equals APPROVED_DESCRIPTION in src/config.ts. */
export const approvedDescription =
  "A small desktop reminder widget for Windows 11. It stays visible after Win+D and nudges you with notifications you can act on.";

const ogImage = { path: "og.png", width: 1200, height: 630, alt: "The Styky reminder panel on a Windows 11 desktop" };

/** Homepage title and description when the agent features are on. Keep them to what ships with the GitHub installer. */
export const agentIndex = {
  title: "Styky – Desktop Reminder Widget for Windows 11 with CLI and MCP Server",
  description:
    "A free, open-source desktop reminder widget for Windows 11 with a local CLI and MCP server. Add reminders from Claude, your terminal or Ctrl+Alt+N.",
};

/** Title and description of a page; the homepage switches on `agent` (the SHOW_AGENT_FEATURES flag). */
export function pageMeta(name, agent) {
  return name === "index" && agent ? { ...pages.index, ...agentIndex } : pages[name];
}

/** One entry per HTML entry file (by file name without .html). `path` is the page URL relative to siteUrl. */
export const pages = {
  index: {
    path: "",
    title: "Styky – Desktop Reminder Widget for Windows 11 (Free, Open Source)",
    description: approvedDescription,
  },
  privacy: {
    path: "privacy",
    title: "Privacy — Styky",
    description:
      "No telemetry. Styky only goes online to check for updates and to activate or remove a Pro key when you click the button.",
  },
  terms: {
    path: "terms",
    title: "Terms — Styky",
    description:
      "Terms for the Styky website and software: free MIT-licensed software, the Pro Supporter license, and purchases through Lemon Squeezy.",
  },
  refund: {
    path: "refund",
    title: "Refund policy — Styky",
    description: "Styky Pro comes with a 14-day, no-questions refund. How to ask, and who processes it.",
  },
  // The /mcp page is agent content: it is only built, listed and linked when the SHOW_AGENT_FEATURES flag is on.
  mcp: {
    path: "mcp",
    title: "Styky MCP Server – Reminders for Claude on Windows",
    description:
      "Styky is a free, local MCP server that lets Claude and other MCP clients add, list and complete reminders that appear on your Windows desktop.",
    agentOnly: true,
  },
  // Not indexable and not in the sitemap: it only gets canonical-free tags (see headTags).
  "404": { path: "404", title: "Page not found — Styky", description: "This page does not exist.", noindex: true },
};

/** Pages that go into sitemap.xml: everything indexable, minus the agent-only pages when the flag is off. */
export function sitemapFor(agent) {
  return Object.entries(pages)
    .filter(([, p]) => !p.noindex && (agent || !p.agentOnly))
    .map(([name, p]) => ({ name, loc: siteUrl + p.path }));
}

/** Campaign landing paths: Cloudflare rewrites them (status 200) to the homepage. Same list as the inline tagging script. */
const campaignPaths = ["sp", "w11", "mcp", "hn", "cai", "registry", "readme", "pro"];

/** The /mcp page replaces its campaign rewrite when it exists (flag on); with the flag off it stays a campaign path. */
export function rewritePaths(agent) {
  return campaignPaths.filter((p) => !(agent && p === "mcp"));
}

/** Content of dist/_redirects (generated at build time; there is no public/_redirects). */
export function redirectsFile(agent) {
  return rewritePaths(agent).map((p) => `/${p.padEnd(9)} /  200`).join(String.fromCharCode(10)) + String.fromCharCode(10);
}

const esc = (s) => String(s).replace(/&/g, "&amp;").replace(/"/g, "&quot;").replace(/</g, "&lt;").replace(/>/g, "&gt;");

/** Canonical, icons, Open Graph and Twitter tags for a page. */
export function headTags(name, agent) {
  const p = pageMeta(name, agent);
  const icons = [
    `<link rel="icon" type="image/svg+xml" href="${base}favicon.svg" />`,
    `<link rel="icon" type="image/png" sizes="48x48" href="${base}favicon-48.png" />`,
    `<link rel="icon" type="image/png" sizes="192x192" href="${base}favicon-192.png" />`,
    `<link rel="apple-touch-icon" href="${base}apple-touch-icon.png" />`,
  ];
  if (p.noindex) return icons.join("\n    ");
  const url = siteUrl + p.path;
  const img = siteUrl + ogImage.path;
  return [
    `<link rel="canonical" href="${esc(url)}" />`,
    ...icons,
    "",
    `<meta property="og:type" content="website" />`,
    `<meta property="og:site_name" content="${esc(siteName)}" />`,
    `<meta property="og:locale" content="en_US" />`,
    `<meta property="og:title" content="${esc(p.title)}" />`,
    `<meta property="og:description" content="${esc(p.description)}" />`,
    `<meta property="og:url" content="${esc(url)}" />`,
    `<meta property="og:image" content="${esc(img)}" />`,
    `<meta property="og:image:width" content="${ogImage.width}" />`,
    `<meta property="og:image:height" content="${ogImage.height}" />`,
    `<meta property="og:image:alt" content="${esc(ogImage.alt)}" />`,
    `<meta name="twitter:card" content="summary_large_image" />`,
    `<meta name="twitter:title" content="${esc(p.title)}" />`,
    `<meta name="twitter:description" content="${esc(p.description)}" />`,
    `<meta name="twitter:image" content="${esc(img)}" />`,
  ].join("\n    ");
}

/** The homepage JSON-LD (one @graph), as a <script> element. Generated so version and URLs cannot go stale. */
export function jsonLdScript({ version, downloadUrl, agent }) {
  // Short factual items, taken from copy that is on the page. The CLI and MCP items only with the flag on.
  const featureList = [
    "Global quick add with natural-language dates",
    "Native Windows notifications with Done and Snooze",
    ...(agent ? ["Local command-line interface (CLI)", "Local MCP server for Claude and other clients that run a local stdio server"] : []),
    "Local SQLite storage with no account",
  ];
  const description = agent ? agentIndex.description : approvedDescription;
  const orgId = `${publisher.url}/#organization`;
  const graph = {
    "@context": "https://schema.org",
    "@graph": [
      {
        "@type": "WebSite",
        "@id": `${siteUrl}#website`,
        name: siteName,
        url: siteUrl,
        inLanguage: "en",
        publisher: { "@id": orgId },
      },
      {
        "@type": "SoftwareApplication",
        "@id": `${siteUrl}#software`,
        name: siteName,
        description,
        url: siteUrl,
        image: siteUrl + ogImage.path,
        operatingSystem: "Windows 10, Windows 11",
        // Google's supported list has no "ProductivityApplication"; UtilitiesApplication is the closest.
        applicationCategory: "UtilitiesApplication",
        featureList,
        softwareVersion: version,
        downloadUrl,
        license: "https://opensource.org/license/mit",
        // Wording of the author entry is not ours to change (see docs/PRODUCT_CONTEXT.md).
        author: { "@type": "Person", name: "Omor Shahriar" },
        publisher: { "@id": orgId },
        offers: { "@type": "Offer", price: "0", priceCurrency: "USD" },
        isPartOf: { "@id": `${siteUrl}#website` },
      },
      { "@type": "Organization", "@id": orgId, name: publisher.name, url: publisher.url },
    ],
  };
  // "<" is escaped so no value can close the script element.
  return `<script type="application/ld+json">${JSON.stringify(graph, null, 2).replace(/</g, "\\u003c")}</script>`;
}

/** Version from src/release.json (refreshed by scripts/release-data.mjs before the build). */
export function readVersion(root) {
  return JSON.parse(fs.readFileSync(`${root}/src/release.json`, "utf8")).version;
}

/** The static installer URL. Read from src/config.ts so it exists in one place only (that file cannot be imported in Node). */
export function readDownloadUrl(root) {
  const m = /downloadUrl:\s*"([^"]+)"/.exec(fs.readFileSync(`${root}/src/config.ts`, "utf8"));
  if (!m) throw new Error("site-meta: downloadUrl not found in src/config.ts");
  return m[1];
}
