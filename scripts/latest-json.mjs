// Writes latest.json for the Tauri updater next to the NSIS installer.
// Usage: node scripts/latest-json.mjs <version> "<notes>"
import { readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";

const [version, notes = ""] = process.argv.slice(2);
if (!version) {
  console.error('usage: node scripts/latest-json.mjs <version> "<notes>"');
  process.exit(1);
}

const dir = join("src-tauri", "target", "release", "bundle", "nsis");
const exe = `Styky_${version}_x64-setup.exe`;
const signature = readFileSync(join(dir, `${exe}.sig`), "utf8").trim();

const manifest = {
  version,
  notes,
  pub_date: new Date().toISOString(),
  platforms: {
    "windows-x86_64": {
      signature,
      url: `https://github.com/omorShahriar/styky/releases/download/v${version}/${exe}`,
    },
  },
};

const out = join(dir, "latest.json");
writeFileSync(out, JSON.stringify(manifest, null, 2) + "\n");
console.log(`wrote ${out}`);
