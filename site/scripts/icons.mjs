// Generates the PNG icons from public/favicon.svg: favicon-48.png, favicon-192.png (Google wants a multiple of 48 px)
// and apple-touch-icon.png (180 px). Run: pnpm icons. The outputs are committed, like og.png.
import sharp from "sharp";
import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import path from "node:path";

const pub = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "../public");
const svg = readFileSync(path.join(pub, "favicon.svg"));

for (const [file, size] of [["favicon-48.png", 48], ["favicon-192.png", 192], ["apple-touch-icon.png", 180]]) {
  // the SVG is 1024 px at the default density; it is rendered once and scaled down
  await sharp(svg)
    .resize(size, size)
    .png({ compressionLevel: 9 })
    .toFile(path.join(pub, file));
  console.log("wrote", file);
}
