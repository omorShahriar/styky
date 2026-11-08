// Microsoft Store listing art from assets/icon.svg → ../../dist-store/ (git-ignored).
// Run from site/: node scripts/store-assets.mjs
import sharp from "sharp";
import { readFileSync, mkdirSync } from "node:fs";
import { fileURLToPath } from "node:url";
import path from "node:path";

const here = path.dirname(fileURLToPath(import.meta.url));
const root = path.resolve(here, "../..");
const out = path.join(root, "dist-store");
mkdirSync(out, { recursive: true });

const iconSvg = readFileSync(path.join(root, "assets/icon.svg"), "utf8");
const inner = iconSvg.replace(/<\?xml[^>]*>/, "").replace(/<svg[^>]*>/, "").replace("</svg>", "");
const font = "Segoe UI Variable Display, Segoe UI, Inter, Helvetica, Arial, sans-serif";

/** Icon only, transparent background, with a little breathing room (Store tile icons). */
async function tile(size) {
  const pad = Math.round(size * 0.08);
  const icon = await sharp(Buffer.from(iconSvg)).resize(size - pad * 2, size - pad * 2).png().toBuffer();
  await sharp({ create: { width: size, height: size, channels: 4, background: { r: 0, g: 0, b: 0, alpha: 0 } } })
    .composite([{ input: icon, top: pad, left: pad }])
    .png()
    .toFile(path.join(out, `tile-${size}x${size}.png`));
}

/** Branded art: dark glassy backdrop, icon, wordmark, optional tagline. */
async function art(w, h, name, { iconFrac, iconY, titleY, tagline }) {
  const s = Math.round(Math.min(w, h) * iconFrac);
  const x = Math.round((w - s) / 2);
  const y = Math.round(h * iconY);
  const title = Math.round(Math.min(w, h) * 0.11);
  const sub = Math.round(Math.min(w, h) * 0.04);
  const svg = `<svg xmlns="http://www.w3.org/2000/svg" width="${w}" height="${h}" viewBox="0 0 ${w} ${h}">
  <defs>
    <linearGradient id="bg" x1="0" y1="0" x2="1" y2="1">
      <stop offset="0" stop-color="#231a3d"/><stop offset="0.55" stop-color="#14121c"/><stop offset="1" stop-color="#0b0b0f"/>
    </linearGradient>
    <radialGradient id="glow" cx="0.5" cy="${(y + s / 2) / h}" r="0.45">
      <stop offset="0" stop-color="#ffc21a" stop-opacity="0.22"/><stop offset="1" stop-color="#ffc21a" stop-opacity="0"/>
    </radialGradient>
  </defs>
  <rect width="${w}" height="${h}" fill="url(#bg)"/>
  <rect width="${w}" height="${h}" fill="url(#glow)"/>
  <svg x="${x}" y="${y}" width="${s}" height="${s}" viewBox="0 0 1024 1024">${inner}</svg>
  ${titleY === null ? "" : `<text x="${w / 2}" y="${Math.round(h * titleY)}" text-anchor="middle" font-family="${font}" font-weight="700"
        font-size="${title}" letter-spacing="${-title * 0.02}" fill="#f5f5f7">Styky</text>`}
  ${tagline ? tagline.map((t, i) => `<text x="${w / 2}" y="${Math.round(h * titleY + title * 0.75 + i * sub * 1.35)}" text-anchor="middle"
        font-family="${font}" font-size="${sub}" fill="#f5f5f7" fill-opacity="0.62">${t}</text>`).join("") : ""}
</svg>`;
  await sharp(Buffer.from(svg)).png().toFile(path.join(out, name));
}

for (const size of [300, 150, 71]) await tile(size);
await art(2160, 2160, "box-art-2160x2160.png", {
  iconFrac: 0.4, iconY: 0.2, titleY: 0.78,
  tagline: ["Always-visible reminders for Windows"],
});
await art(1440, 2160, "poster-art-1440x2160.png", {
  iconFrac: 0.5, iconY: 0.22, titleY: 0.68,
  tagline: ["The reminders you can’t", "afford to forget,", "always on your desktop."],
});
// 16:9 Super hero art: the Store forbids the product title here, so icon only.
await art(3840, 2160, "super-hero-art-3840x2160.png", { iconFrac: 0.42, iconY: 0.29, titleY: null });
console.log(`wrote store art to ${out}`);
