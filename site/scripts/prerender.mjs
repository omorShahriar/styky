// Prerenders the legal pages (refund, terms, privacy) into dist/*.html, so the raw HTML response has the page's own
// content and does not depend on JavaScript. Runs after `vite build`. The homepage stays a client-rendered page.
import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { createServer } from "vite";

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const PAGES = ["refund", "terms", "privacy"];

const server = await createServer({ root, appType: "custom", logLevel: "error", server: { middlewareMode: true } });
try {
  const { render } = await server.ssrLoadModule("/src/legal-ssr.tsx");
  for (const name of PAGES) {
    const file = path.join(root, "dist", `${name}.html`);
    const html = fs.readFileSync(file, "utf8");
    const mount = '<div id="root"></div>';
    if (!html.includes(mount)) throw new Error(`${name}.html: empty mount point not found`);
    fs.writeFileSync(file, html.replace(mount, `<div id="root">${render(name)}</div>`));
    console.log(`prerendered ${name}.html`);
  }
} finally {
  await server.close();
}
