import { defineConfig, type Plugin } from "vite";
import react from "@vitejs/plugin-react";
import path from "node:path";
import { fileURLToPath } from "node:url";

const here = path.dirname(fileURLToPath(import.meta.url));
const appSrc = path.resolve(here, "../src");
const mock = (file: string) => path.resolve(here, "src/mock", file);

/** Public base path. "/" on Cloudflare Pages (styky.pages.dev or a custom domain); "/styky/" only for GitHub Pages. */
const base = process.env.SITE_BASE || "/";
/** Absolute site URL, used for canonical / OG tags. */
const siteUrl = (process.env.SITE_URL || "https://styky.pages.dev/").replace(/\/?$/, "/");

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

const exact = (name: string) => new RegExp(`^${name.replace(/[/@.-]/g, "\$&")}$`);

// https://vite.dev/config/
export default defineConfig({
  base,
  plugins: [react(), mockApi(), htmlVars()],
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
  build: { target: "es2022", sourcemap: false },
});
