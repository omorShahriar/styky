// Writes latest.json for the Tauri updater next to the NSIS installer, and an unversioned copy of the installer
// (Styky_x64-setup.exe) so the website can link to .../releases/latest/download/Styky_x64-setup.exe forever.
// Upload both installer names to every release; latest.json keeps pointing at the versioned one.
// Usage: node scripts/latest-json.mjs <version> "<notes>"
import { copyFileSync, existsSync, readFileSync, statSync, writeFileSync } from "node:fs";
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
      url: `https://github.com/smallhingelabs/styky/releases/download/v${version}/${exe}`,
    },
  },
};

const out = join(dir, "latest.json");
writeFileSync(out, JSON.stringify(manifest, null, 2) + "\n");
console.log(`wrote ${out}`);

// Unversioned copy for the static download link on the website.
const stable = join(dir, "Styky_x64-setup.exe");
if (!existsSync(join(dir, exe))) {
  console.error(`missing ${join(dir, exe)}`);
  process.exit(1);
}
copyFileSync(join(dir, exe), stable);
console.log(`wrote ${stable} (copy of ${exe})`);

// Everything `gh release create` needs, so nothing is forgotten.
console.log("\nUpload these four files to the release:");
for (const name of [exe, "Styky_x64-setup.exe", `${exe}.sig`, "latest.json"]) {
  const p = join(dir, name);
  if (!existsSync(p)) {
    console.error(`  MISSING ${p}`);
    process.exitCode = 1;
    continue;
  }
  console.log(`  ${p} (${statSync(p).size} bytes)`);
}
