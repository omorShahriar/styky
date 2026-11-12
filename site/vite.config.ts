import { defineConfig, type Plugin } from "vite";
import react from "@vitejs/plugin-react";
import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { base, headTags, jsonLdScript, pageMeta, pages, readDownloadUrl, rewritePaths, readVersion, siteName, siteUrl } from "./site-meta.mjs";

const here = path.dirname(fileURLToPath(import.meta.url));
const appSrc = path.resolve(here, "../src");
const mock = (file: string) => path.resolve(here, "src/mock", file);

// base and siteUrl live in site-meta.mjs, with the rest of the head metadata.

/**
 * C8: the CLI and MCP server shipped in v0.2.0 and the release is public, so the agent copy is on. The flag now only
 * exists as an off switch: everything that talks about agents, Claude, MCP, the CLI, the terminal or git hooks is behind
 * it and is absent from the build (HTML, JS and head metadata) while it is false. Build once with
 * SHOW_AGENT_FEATURES=false to get the plain site. The metadata variants live in site-meta.mjs (agentIndex and the
 * plain homepage entry); see site/README.md.
 */
const SHOW_AGENT_FEATURES_DEFAULT = true;
const showAgentFeatures = process.env.SHOW_AGENT_FEATURES
  ? process.env.SHOW_AGENT_FEATURES === "true"
  : SHOW_AGENT_FEATURES_DEFAULT;

/** The app's real `src/lib/api.ts` (Tauri invoke wrappers) is swapped for the in-memory mock. */
function mockApi(): Plugin {
  const real = path.join(appSrc, "lib", "api.ts");
  return {
    name: "styky-mock-api",
    enforce: "pre",
    async resolveId(source, importer, opts) {
      if (!importer || !/(^|\/)api$/.test(source)) return null;
      const r = await this.resolve(source, importer, { ...opts, skipSelf: true });
      return r && path.normalize(r.id) === path.normalize(real) ? mock("api.ts") : null;
    },
  };
}

/** Fills __TITLE__, __DESCRIPTION__, __HEAD__ (canonical, icons, OG, Twitter) and __JSONLD__ from site-meta.mjs, per entry file. */
function htmlVars(): Plugin {
  return {
    name: "styky-html-vars",
    transformIndexHtml(html, ctx) {
      const file = path.basename(ctx.filename ?? "index.html", ".html");
      const name = (file in pages ? file : "index") as keyof typeof pages;
      const page = pageMeta(name, showAgentFeatures);
      const esc = (s: string) => s.replace(/&/g, "&amp;").replace(/"/g, "&quot;").replace(/</g, "&lt;");
      return html
        .replaceAll("__TITLE__", esc(page.title))
        .replaceAll("__DESCRIPTION__", esc(page.description))
        .replaceAll("__HEAD__", headTags(name, showAgentFeatures))
        .replaceAll("__JSONLD__", () => jsonLdScript({ version: readVersion(here), downloadUrl: readDownloadUrl(here), agent: showAgentFeatures }))
        .replaceAll("__SITE_NAME__", siteName)
        .replaceAll("__SITE_URL__", siteUrl)
        .replaceAll("__BASE__", base);
    },
  };
}

/**
 * Mirror Cloudflare Pages in dev and preview. Clean URLs for the legal pages (refund.html is served at /refund). In
 * preview, also the `200` campaign rewrites (site-meta.mjs, written to dist/_redirects by scripts/prerender.mjs; campaign paths -> /) and a real 404 (dist/404.html) for any
 * other extension-less path; Pages does the same once a top-level 404.html exists (it then drops its SPA fallback).
 */
const LEGAL = ["refund", "terms", "privacy", ...(showAgentFeatures ? ["mcp"] : [])];
function cleanLegalUrls(): Plugin {
  const legal = (req: { url?: string }) => {
    const m = /^\/(refund|terms|privacy|mcp)\/?(\?.*)?$/.exec(req.url ?? "");
    if (m && LEGAL.includes(m[1])) req.url = `/${m[1]}.html${m[2] ?? ""}`;
  };
  return {
    name: "styky-clean-legal-urls",
    configureServer: (s) => void s.middlewares.use((req, _res, next) => (legal(req), next())),
    configurePreviewServer(s) {
      const outDir = path.resolve(here, s.config.build.outDir);
      const rewrites = new Set(rewritePaths(showAgentFeatures).map((p) => `/${p}`));
      s.middlewares.use((req, res, next) => {
        legal(req);
        const p = (req.url ?? "/").split("?")[0];
        if (rewrites.has(p)) req.url = "/";
        else if (p !== "/" && !/\.[a-z0-9]+$/i.test(p) && !LEGAL.some((n) => p === `/${n}`)) {
          const page = path.join(outDir, "404.html");
          if (fs.existsSync(page)) {
            res.statusCode = 404;
            res.setHeader("Content-Type", "text/html; charset=utf-8");
            return void res.end(fs.readFileSync(page));
          }
        }
        next();
      });
    },
  };
}

const exact = (name: string) => new RegExp(`^${name.replace(/[/@.-]/g, "\$&")}$`);

// https://vite.dev/config/
export default defineConfig({
  base,
  define: { __SHOW_AGENT_FEATURES__: JSON.stringify(showAgentFeatures) },
  plugins: [react(), mockApi(), htmlVars(), cleanLegalUrls()],
  resolve: {
    alias: [
      { find: exact("@tauri-apps/api/core"), replacement: mock("tauri-core.ts") },
      { find: exact("@tauri-apps/api/event"), replacement: mock("tauri-event.ts") },
      { find: exact("@tauri-apps/api/window"), replacement: mock("tauri-window.ts") },
      { find: exact("@tauri-apps/api/dpi"), replacement: mock("tauri-dpi.ts") },
      { find: exact("@tauri-apps/plugin-dialog"), replacement: mock("plugin-dialog.ts") },
      { find: exact("@tauri-apps/plugin-opener"), replacement: mock("plugin-opener.ts") },
      { find: exact("@tauri-apps/plugin-clipboard-manager"), replacement: mock("plugin-clipboard.ts") },
      { find: "@app", replacement: appSrc },
    ],
    // The app source lives outside site/: make every shared package resolve to site/node_modules
    // so there is exactly one copy of React, motion and zustand.
    dedupe: [
      "react",
      "react-dom",
      "motion",
      "zustand",
      "lucide-react",
      "react-markdown",
      "remark-gfm",
      "rehype-sanitize",
      "chrono-node",
      "@fontsource-variable/inter",
    ],
  },
  server: { fs: { allow: [path.resolve(here, "..")] } },
  build: {
    target: "es2022",
    sourcemap: false,
    rollupOptions: {
      input: {
        main: path.resolve(here, "index.html"),
        refund: path.resolve(here, "refund.html"),
        terms: path.resolve(here, "terms.html"),
        privacy: path.resolve(here, "privacy.html"),
        "404": path.resolve(here, "404.html"),
        // agent content: not built at all when the flag is off
        ...(showAgentFeatures ? { mcp: path.resolve(here, "mcp.html") } : {}),
      },
    },
  },
});
