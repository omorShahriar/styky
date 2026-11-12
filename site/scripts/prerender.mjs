// Runs after `vite build`:
//  - prerenders the homepage (index.html), the legal pages (refund, terms, privacy) and 404.html into dist/*.html, so the
//    raw HTML response has the page's own content and does not depend on JavaScript. The homepage hydrates in the browser
//    (src/main.tsx); the demo is mounted after hydration and only its skeleton is in the HTML.
//  - writes dist/sitemap.xml and dist/robots.txt from siteUrl (site-meta.mjs).
import { execFileSync } from "node:child_process";
import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { createServer } from "vite";
import { approvedDescription, base, redirectsFile, siteUrl, sitemapFor } from "../site-meta.mjs";

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const dist = path.join(root, "dist");
const mount = '<div id="root"></div>';

function inject(file, markup) {
  const f = path.join(dist, file);
  const html = fs.readFileSync(f, "utf8");
  if (!html.includes(mount)) throw new Error(`${file}: empty mount point not found`);
  fs.writeFileSync(f, html.replace(mount, () => `<div id="root">${markup}</div>`));
  console.log(`prerendered ${file}`);
}

/** YYYY-MM-DD of the last commit that touched `paths` (the checkout Cloudflare builds from has git history for HEAD), else today. */
function lastCommitDate(paths) {
  try {
    const d = execFileSync("git", ["log", "-1", "--format=%cs", "--", ...paths], { cwd: root, encoding: "utf8" }).trim();
    if (/^\d{4}-\d{2}-\d{2}$/.test(d)) return d;
  } catch {}
  return new Date().toISOString().slice(0, 10);
}

const server = await createServer({ root, appType: "custom", logLevel: "error", server: { middlewareMode: true } });
try {
  const config = await server.ssrLoadModule("/src/config.ts");
  if (config.APPROVED_DESCRIPTION !== approvedDescription) {
    throw new Error("APPROVED_DESCRIPTION in src/config.ts differs from approvedDescription in site-meta.mjs: keep them identical");
  }

  const { renderHome } = await server.ssrLoadModule("/src/home-ssr.tsx");
  inject("index.html", await renderHome());
  // The hero text is the LCP and is set in Inter: preload the latin subset so the font does not wait for the stylesheet.
  const latin = fs.readdirSync(path.join(dist, "assets")).find((f) => /^inter-latin-wght-normal-.*.woff2$/.test(f));
  if (!latin) throw new Error("inter-latin woff2 not found in dist/assets");
  const idx = path.join(dist, "index.html");
  fs.writeFileSync(
    idx,
    fs.readFileSync(idx, "utf8").replace("</head>", () => `  <link rel="preload" as="font" type="font/woff2" crossorigin href="${base}assets/${latin}" />
  </head>`),
  );
  const { render } = await server.ssrLoadModule("/src/legal-ssr.tsx");
  for (const name of ["refund", "terms", "privacy", "404"]) inject(`${name}.html`, render(name));
  // The flag comes from vite.config.ts (the server above loaded it), so the build and this step always agree.
  const agent = config.SHOW_AGENT_FEATURES;
  if (agent) inject("mcp.html", (await server.ssrLoadModule("/src/mcp-ssr.tsx")).render());
  else if (fs.existsSync(path.join(dist, "mcp.html"))) throw new Error("mcp.html is in dist but the agent flag is off");

  // Cloudflare Pages rewrites for the campaign paths. /mcp is one of them only while the real page does not exist.
  fs.writeFileSync(path.join(dist, "_redirects"), redirectsFile(agent));

  // "November 9, 2026" -> 2026-11-09 (the legal pages' own "Last updated").
  const legalDate = new Date(`${config.legalUpdated} UTC`).toISOString().slice(0, 10);
  const homeDate = lastCommitDate(["."]);
  const mcpDate = lastCommitDate(["mcp.html", "src/mcp.tsx", "src/mcp-ssr.tsx", "src/sections/McpPage.tsx", "src/styles/mcp.css"]);
  const lastmod = (name) => (name === "index" ? homeDate : name === "mcp" ? mcpDate : legalDate);
  const urls = sitemapFor(agent)
    .map((p) => `  <url>\n    <loc>${p.loc}</loc>\n    <lastmod>${lastmod(p.name)}</lastmod>\n  </url>`)
    .join("\n");
  fs.writeFileSync(
    path.join(dist, "sitemap.xml"),
    `<?xml version="1.0" encoding="UTF-8"?>\n<urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">\n${urls}\n</urlset>\n`,
  );
  fs.writeFileSync(path.join(dist, "robots.txt"), `User-agent: *\nAllow: /\n\nSitemap: ${siteUrl}sitemap.xml\n`);
  console.log("wrote sitemap.xml, robots.txt");
} finally {
  await server.close();
}
