// Generates public/og.png (1200x630) from an inline SVG. Run: pnpm og
import sharp from "sharp";
import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import path from "node:path";

const here = path.dirname(fileURLToPath(import.meta.url));
const out = path.resolve(here, "../public/og.png");
const icon = readFileSync(path.resolve(here, "../public/favicon.svg"), "utf8")
  .replace(/<\?xml[^>]*>/, "")
  .replace(/<svg[^>]*>/, "")
  .replace("</svg>", "");

const font = "Segoe UI, Inter, Helvetica, Arial, sans-serif";

const rows = [
  { y: 0, t: "Rotate the production API keys", sub: "Overdue · 22 min ago", tag: "security", urgent: true },
  { y: 70, t: "Release v2.3", sub: "In 2h", tag: "api", urgent: true },
  { y: 140, t: "Reply to Dana about the contract", sub: "In 5h", tag: "work" },
  { y: 210, t: "Renew the SSL certificate", sub: "Tomorrow 9:00 AM", tag: "ops" },
];

const row = (r) => `
  <g transform="translate(0 ${r.y})">
    <circle cx="26" cy="26" r="9" fill="none" stroke="rgba(255,255,255,0.45)" stroke-width="1.6"/>
    <text x="50" y="25" font-family="${font}" font-size="19" font-weight="600" fill="rgba(255,255,255,0.92)">${r.t}</text>
    <text x="50" y="47" font-family="${font}" font-size="14" fill="rgba(255,255,255,0.55)">${r.tag}  ·  ${r.sub}</text>
    ${r.urgent ? '<circle cx="408" cy="19" r="4" fill="#ff6961"/>' : ""}
  </g>`;

const svg = `
<svg xmlns="http://www.w3.org/2000/svg" width="1200" height="630" viewBox="0 0 1200 630">
  <defs>
    <linearGradient id="bg" x1="0" y1="0" x2="1" y2="1">
      <stop offset="0" stop-color="#0a0f2e"/><stop offset="0.55" stop-color="#1b0f45"/><stop offset="1" stop-color="#2a0f3a"/>
    </linearGradient>
    <radialGradient id="b1"><stop offset="0" stop-color="#2a5bff" stop-opacity="0.85"/><stop offset="1" stop-color="#2a5bff" stop-opacity="0"/></radialGradient>
    <radialGradient id="b2"><stop offset="0" stop-color="#8b3dff" stop-opacity="0.8"/><stop offset="1" stop-color="#8b3dff" stop-opacity="0"/></radialGradient>
    <radialGradient id="b3"><stop offset="0" stop-color="#0ea5e9" stop-opacity="0.6"/><stop offset="1" stop-color="#0ea5e9" stop-opacity="0"/></radialGradient>
    <filter id="sh" x="-20%" y="-20%" width="140%" height="150%"><feDropShadow dx="0" dy="24" stdDeviation="28" flood-color="#000" flood-opacity="0.45"/></filter>
  </defs>
  <rect width="1200" height="630" fill="url(#bg)"/>
  <ellipse cx="260" cy="120" rx="520" ry="360" fill="url(#b1)"/>
  <ellipse cx="1040" cy="520" rx="520" ry="380" fill="url(#b2)"/>
  <ellipse cx="640" cy="720" rx="520" ry="260" fill="url(#b3)"/>

  <g transform="translate(80 84)">
    <svg width="64" height="64" viewBox="0 0 1024 1024">${icon}</svg>
    <text x="84" y="46" font-family="${font}" font-size="38" font-weight="700" fill="#fff" letter-spacing="-1">Styky</text>
  </g>
  <text font-family="${font}" font-size="54" font-weight="700" fill="#fff" letter-spacing="-2">
    <tspan x="80" y="222">Reminders you control</tspan>
    <tspan x="80" y="286">from Claude, your</tspan>
    <tspan x="80" y="350">terminal and git hooks.</tspan>
  </text>
  <text x="80" y="408" font-family="Consolas, Cascadia Mono, monospace" font-size="23" fill="rgba(255,255,255,0.86)">$ styky add &quot;Deploy api fri 5pm #infra !!&quot;</text>
  <text x="80" y="450" font-family="${font}" font-size="24" fill="rgba(255,255,255,0.72)">A free, open-source Windows reminder widget.</text>

  <g transform="translate(80 500)">
    <rect width="258" height="60" rx="30" fill="#7d63ff"/>
    <text x="129" y="38" text-anchor="middle" font-family="${font}" font-size="22" font-weight="600" fill="#fff">Free for Windows</text>
  </g>

  <g transform="translate(700 92)" filter="url(#sh)">
    <rect width="440" height="446" rx="18" fill="rgba(30,30,36,0.62)" stroke="rgba(255,255,255,0.14)"/>
    <text x="26" y="48" font-family="${font}" font-size="22" font-weight="600" fill="rgba(255,255,255,0.92)">Styky</text>
    <text x="26" y="72" font-family="${font}" font-size="14" fill="rgba(255,255,255,0.55)">3 due today</text>
    <line x1="14" y1="90" x2="426" y2="90" stroke="rgba(255,255,255,0.1)"/>
    <g transform="translate(14 104)">${rows.map(row).join("")}</g>
  </g>
</svg>`;

await sharp(Buffer.from(svg)).png({ compressionLevel: 9 }).toFile(out);
console.log("wrote", out);
