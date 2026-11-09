import { defineConfig, type Plugin } from "vite";
import react from "@vitejs/plugin-react";
import path from "node:path";
import { fileURLToPath } from "node:url";

const here = path.dirname(fileURLToPath(import.meta.url));
const appSrc = path.resolve(here, "../src");
const mock = (file: string) => path.resolve(here, "src/mock", file);

/** Public base path. "/" on Cloudflare Pages (custom domain); "/styky/" only for GitHub Pages. */
const base = process.env.SITE_BASE || "/";
/** Absolute site URL, used for canonical / OG tags. */
const siteUrl = (process.env.SITE_URL || "https://styky.smallhingelabs.com/").replace(/\/?$/, "/");

/**
 * C8 (LOCKED): no public MCP or CLI claims until a release that contains them is published (v0.2.0).
 * Everything that talks about agents, Claude, MCP, the CLI, the terminal or git hooks is behind this one flag and is
 * absent from the build (HTML and JS) while it is false. After v0.2.0 is published, change `false` to `true` below
 * (or build once with SHOW_AGENT_FEATURES=true). The head metadata in index.html is separate: see docs/RELEASING.md.
 */
const SHOW_AGENT_FEATURES_DEFAULT = false;
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

function htmlVars(): Plugin {
  return {
    name: "styky-html-vars",
    transformIndexHtml: (html) => html.replaceAll("__SITE_URL__", siteUrl).replaceAll("__BASE__", base),
  };
}

/** Clean URLs for the legal pages: Cloudflare Pages serves refund.html at /refund; mirror that in dev and preview. */
const LEGAL = ["refund", "terms", "privacy"];
function cleanLegalUrls(): Plugin {
  const rewrite = (req: { url?: string }, _res: unknown, next: () => void) => {
    const m = /^\/(refund|terms|privacy)\/?(\?.*)?$/.exec(req.url ?? "");
    if (m && LEGAL.includes(m[1])) req.url = `/${m[1]}.html${m[2] ?? ""}`;
    next();
  };
  return {
    name: "styky-clean-legal-urls",
    configureServer: (s) => void s.middlewares.use(rewrite),
    configurePreviewServer: (s) => void s.middlewares.use(rewrite),
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
      },
    },
  },
});
