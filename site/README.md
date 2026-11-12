# Styky website

The landing page for [Styky](https://github.com/smallhingelabs/styky): a static Vite + React + TypeScript app, deployed
to **Cloudflare Pages** at `https://styky.smallhingelabs.com/`.

The hero is a **live demo of the real app UI**. `site/` imports `src/app/Panel.tsx`, `src/app/QuickAdd.tsx`, the
components and `src/lib/nlp.ts` straight from the app (read-only) and replaces everything that talks to Tauri.

## Develop

```bash
cd site
pnpm install --ignore-workspace   # the site has its own package.json; the repo root is not a workspace
pnpm dev                          # http://localhost:5173/
pnpm build                        # -> site/dist
pnpm preview                      # serves dist at http://localhost:4173/
pnpm typecheck
pnpm og                           # regenerate public/og.png (the social card)
pnpm icons                        # regenerate the PNG icons from public/favicon.svg
```

## How the demo mocks the app

`vite.config.ts` aliases, so no app file is edited:

| Real import | Replaced by |
|---|---|
| `src/lib/api.ts` (Tauri `invoke` wrappers, matched by a resolver plugin) | `src/mock/api.ts` |
| `@tauri-apps/api/core` / `event` / `window` / `dpi` | `src/mock/tauri-*.ts` |
| `@tauri-apps/plugin-dialog` / `opener` / `clipboard-manager` | `src/mock/plugin-*.ts` |

- `src/mock/db.ts` is an in-memory stand-in for the SQLite store: list (same ordering as `sort.rs`), create, update,
  complete (repeating items roll forward), snooze, delete/undelete, restore, reorder. Every write emits
  `items-changed` on a small event bus (`mock/events.ts`) that backs `listen()`.
- `lib/theme.ts` would delete the page theme; `mock/site-theme.ts` makes the mocked `theme` setting mirror the site's.
- Nothing is persisted. "Reset demo" re-seeds the data.
- The page's own keyboard shortcuts stay inert until the visitor clicks into the demo (`Demo.tsx`), so Space and the
  arrow keys never hijack normal scrolling.
- The snooze popover is `position: fixed`; when the stage is scaled down (phones) its coordinates are converted back.
- Heavy code (the demo and the type-along section) is lazy-loaded so the first paint is the text.

If the app's UI changes, the demo follows automatically. If a new Tauri command or plugin is used by `Panel` or its
components, add it to `src/mock/` or the build will fail to resolve it.

## Deploy (Cloudflare Pages)

Cloudflare builds the site from this repo on every push to `main` (and builds a preview URL for other branches).
No GitHub Actions are involved.

**One-time setup:** Cloudflare dashboard → **Workers & Pages → Create → Pages → Connect to Git**, then pick
`smallhingelabs/styky` and use these settings:

| Setting | Value |
|---|---|
| Project name | `styky` (its default `*.pages.dev` address redirects to the custom domain) |
| Production branch | `main` |
| Framework preset | None |
| Root directory | `site` |
| Build command | `pnpm build` |
| Build output directory | `dist` |
| Environment variables | `NODE_VERSION` = `22`, `PNPM_VERSION` = `10.15.1` |

Cloudflare clones the whole repo, so the demo's read-only imports from `../src` resolve. The site's own dependencies
cover them; this was checked with a clean clone that had no root `node_modules`. `public/_headers` sets long-lived caching
for hashed assets.

The canonical and Open Graph tags default to `https://styky.smallhingelabs.com/`. Set the env var `SITE_URL` (with trailing
slash) to override it, for example on a preview build.

## Release data (version, stars)

The browser never calls a GitHub API. `pnpm build` first runs `scripts/release-data.mjs`, which reads the latest release tag
and the star count from the GitHub API and writes them to `src/release.json`. That file is committed, so a build that cannot
reach GitHub, or is rate-limited (60 requests an hour per IP without a token), keeps the committed values and still succeeds.
Set `GITHUB_TOKEN` in the build environment to raise the limit. The values are baked into the page, so they are as fresh as the
last deploy.

## Pro teaser

Until Pro goes on sale the pricing section is a four-line teaser (`proTeaser` in `src/config.ts`): no Buy button, no checkout
link, no email form. The full /pro page replaces it later.

## Network statement and legal pages

The one canonical network statement is `NETWORK_STATEMENT` in `src/config.ts`; keep it byte-identical with `docs/PRIVACY.md` and
the README. `/refund`, `/terms` and `/privacy` are separate pages (`refund.html`, `terms.html`, `privacy.html`, rendered by
`src/legal.tsx` from `src/sections/LegalPages.tsx`); Cloudflare Pages serves them at the clean URLs. `pnpm build` runs
`scripts/prerender.mjs` after `vite build`, which writes each page's markup into `dist/<page>.html`, so the raw HTML has the page's
own content, title and canonical without running JavaScript.

## Prerendering, hydration and the 404 page

`pnpm build` is `vite build` followed by `scripts/prerender.mjs`, which writes the rendered markup into `dist/index.html`
(`src/home-ssr.tsx`, React's `prerender`), the three legal pages and `404.html` (`src/legal-ssr.tsx`), and then generates
`sitemap.xml` and `robots.txt`.

- **Homepage:** the whole page is in the HTML, including the lazy-loaded Grammar section (`prerender` waits for it). The live demo is
  not: `DemoSlot` in `sections/Chrome.tsx` renders the skeleton on the server and for the first client render, and mounts the demo
  from an effect after hydration, so the browser-only mocks never run on the server. Nothing may touch `window`, `document` or
  `localStorage` at module scope or during render; the theme toggle reads `<html data-theme>` through `useSyncExternalStore`.
- **Hydration:** `src/main.tsx` hydrates when `#root` has markup and uses `createRoot` otherwise (the dev server serves an empty root).
  Scroll-reveal blocks are hidden by CSS until JavaScript runs, so `index.html` has a `<noscript>` style that shows them.
- **404:** `404.html` (rendered by `NotFound` in `LegalPages.tsx`, `noindex`) makes Cloudflare Pages answer unknown paths with a real
  404 instead of the homepage; a top-level `404.html` turns off its SPA fallback. The `200` rewrites in `public/_redirects` are the
  only other exception (generated into `dist/_redirects` by the prerender step). `vite preview` mirrors both.
- **Sitemap and robots:** generated from `siteUrl`, never hand-written. The sitemap lists the homepage and the legal pages (each `<loc>`
  is that page's canonical URL), not the campaign paths or `/404`. `lastmod` is the last git commit date for the homepage and the
  legal pages' own "Last updated" date.
- **Preview deployments:** `public/_headers` sends `X-Robots-Tag: noindex` for `*.pages.dev` hosts only, not the custom domain.

## Head metadata and structured data

All of it comes from `site-meta.mjs`: titles and descriptions per page, the canonical, icon, Open Graph and Twitter tags, the sitemap
URLs and the JSON-LD (`WebSite`, `SoftwareApplication`, `Organization` in one `@graph`; version from `src/release.json`, download URL
from `src/config.ts`). The `*.html` entries only hold the placeholders `__TITLE__`, `__DESCRIPTION__`, `__HEAD__` and `__JSONLD__`, filled by
the `htmlVars` plugin in `vite.config.ts`. The prerender step fails if `APPROVED_DESCRIPTION` in `src/config.ts` differs from the
description in `site-meta.mjs`. No ratings, reviews or FAQ markup, and no Store link until the listing is live.

Icons: `public/favicon.svg` plus `favicon-48.png`, `favicon-192.png` and `apple-touch-icon.png`, generated by `pnpm icons`
(`scripts/icons.mjs`, sharp) and committed.

## The /mcp page

`mcp.html` (rendered by `src/mcp.tsx` / `src/sections/McpPage.tsx`, prerendered through `src/mcp-ssr.tsx`, styled by `src/styles/mcp.css`) is the
setup page for the MCP server: client snippets and the seven tools with their parameters. It is agent content, so it follows the C8 flag:

- **Flag on (default):** it is built, prerendered into `dist/mcp.html`, served at `/mcp`, listed in `sitemap.xml`, and linked from the Connect
  section ("Full setup and tool list") and the footer ("MCP server"). The nav item "Claude & CLI" still points at `#connect`.
- **Flag off:** `mcp.html` is not a build input, so there is no `dist/mcp.html`, no sitemap entry and no link, and `/mcp` is a campaign path
  to the homepage again.
- **Head:** title and description are the `mcp` entry in `site-meta.mjs`; `mcp.html` only holds the placeholders, its `BreadcrumbList`
  JSON-LD and the channel-tagging script (which yields `site-mcp` at `/mcp`).
- **Rewrites:** `dist/_redirects` is generated by `scripts/prerender.mjs` from the campaign-path list in `site-meta.mjs` (there is no
  `public/_redirects`); `/mcp` is left out of it while the real page exists. `vite preview` uses the same list.

## Agent / MCP / CLI copy (rule C8)

The CLI and MCP server shipped in v0.2.0 and the release is public, so the agent copy is **on**: `SHOW_AGENT_FEATURES_DEFAULT` in
`vite.config.ts` is `true` (read in the app as `SHOW_AGENT_FEATURES` from `src/config.ts`). The flag now exists as an off switch.
While it is `false` the sections are absent from the build, not just hidden with CSS, the places where a sentence carried a claim show
the approved plain wording, and the head metadata falls back too. Build once with `SHOW_AGENT_FEATURES=false pnpm build` to get that
plain site (a flag-off build has no CLI, MCP or Claude wording in the HTML or metadata apart from the campaign-path list in the inline
channel-tagging script).

The head metadata has two variants, both in `site-meta.mjs`, chosen by the same flag:

- flag on: `agentIndex` (title and description), and the JSON-LD `featureList` includes the CLI and MCP items;
- flag off: the homepage entry in `pages` (the earlier title and `approvedDescription`, which must equal `APPROVED_DESCRIPTION` in
  `src/config.ts`) and a `featureList` without them.

Metadata stays within what ships: the CLI and MCP server come with the GitHub installer, not the Microsoft Store build; the MCP server is
local stdio (no remote clients); no npm, `npx`, MCP Registry or bundle mentions until those exist; no Pro features or prices.

## Files

- `site-meta.mjs`: titles, descriptions, canonical/OG tags, JSON-LD, sitemap URLs, campaign rewrites (`_redirects`)
- `mcp.html`, `src/sections/McpPage.tsx`: the /mcp page (agent flag on only)
- `src/config.ts`: repo links, static download URL, network statement, Pro teaser copy
- `src/release.json`: latest release version and stars (refreshed at build time)
- `src/sections/`: page sections (`Chrome` = nav, hero; `Shell` = footer and legal header; `Content`; `Grammar`; `Pricing` = the Pro teaser; `LegalPages`)
- `src/demo/`: the stage (`Demo.tsx`), the type-along bar (`TypeBar.tsx`), the toast mock
- `src/styles/site.css`: page styling (the app's own `tokens.css` is imported, so colours, radii and springs match)
- `public/og.png` is generated by `scripts/og.mjs`; the PNG icons by `scripts/icons.mjs`; `public/favicon.svg` is a copy of `assets/icon.svg`

Every download button points at the static URL `https://github.com/smallhingelabs/styky/releases/latest/download/Styky_x64-setup.exe`.
Each release uploads the installer under that unversioned name too (see `docs/RELEASING.md`), so nothing needs a redeploy.
