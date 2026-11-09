# Styky website

The landing page for [Styky](https://github.com/omorShahriar/styky): a static Vite + React + TypeScript app, deployed
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
`omorShahriar/styky` and use these settings:

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
`src/legal.tsx` from `src/sections/LegalPages.tsx`); Cloudflare Pages serves them at the clean URLs.

## Files

- `src/config.ts`: repo links, static download URL, network statement, Pro teaser copy
- `src/release.json`: latest release version and stars (refreshed at build time)
- `src/sections/`: page sections (`Chrome` = nav, hero; `Shell` = footer and legal header; `Content`; `Grammar`; `Pricing` = the Pro teaser; `LegalPages`)
- `src/demo/`: the stage (`Demo.tsx`), the type-along bar (`TypeBar.tsx`), the toast mock
- `src/styles/site.css`: page styling (the app's own `tokens.css` is imported, so colours, radii and springs match)
- `public/og.png` is generated by `scripts/og.mjs`; `public/favicon.svg` is a copy of `assets/icon.svg`

Every download button points at the static URL `https://github.com/omorShahriar/styky/releases/latest/download/Styky_x64-setup.exe`.
Each release uploads the installer under that unversioned name too (see `docs/RELEASING.md`), so nothing needs a redeploy.
