// Growth snapshot from GitHub: stars, repo traffic, and installer downloads per release.
// Usage: node scripts/stats.mjs          (needs the GitHub CLI `gh`, logged in as a repo admin for traffic)
//        node scripts/stats.mjs --log    (also appends one line to private/stats.csv, which is git-ignored)
//
// Website visits come from Cloudflare Web Analytics; Microsoft Store installs come from Partner Center → Analytics.
// See private/GROWTH.md (git-ignored) for the 60-day validation gates these numbers feed.
import { execFileSync } from "node:child_process";
import { appendFileSync, existsSync } from "node:fs";

const REPO = "smallhingelabs/styky";
const gh = (path) => JSON.parse(execFileSync("gh", ["api", path], { encoding: "utf8" }));
const tryGh = (path) => {
  try {
    return gh(path);
  } catch {
    return null; // traffic endpoints need push access; skip quietly otherwise
  }
};

const repo = gh(`repos/${REPO}`);
const releases = gh(`repos/${REPO}/releases?per_page=100`);
const views = tryGh(`repos/${REPO}/traffic/views`);
const clones = tryGh(`repos/${REPO}/traffic/clones`);
const referrers = tryGh(`repos/${REPO}/traffic/popular/referrers`) ?? [];

// Count only installers: .sig and latest.json are fetched by the updater and by tooling, not by people.
const isInstaller = (name) => /\.(exe|msi|msix)$/i.test(name);
let total = 0;
const perRelease = releases.map((r) => {
  const n = r.assets.filter((a) => isInstaller(a.name)).reduce((s, a) => s + a.download_count, 0);
  total += n;
  return { tag: r.tag_name, published: r.published_at?.slice(0, 10), downloads: n };
});

const today = new Date().toISOString().slice(0, 10);
console.log(`Styky growth snapshot, ${today}`);
console.log(`  stars ${repo.stargazers_count} · forks ${repo.forks_count} · watchers ${repo.subscribers_count}`);
if (views) console.log(`  repo views (14d): ${views.count} (${views.uniques} unique) · clones (14d): ${clones?.count ?? "?"}`);
console.log(`  installer downloads, all releases: ${total}`);
for (const r of perRelease) console.log(`    ${r.tag.padEnd(8)} ${r.published ?? ""}  ${r.downloads}`);
if (referrers.length) {
  console.log("  top referrers (14d):");
  for (const r of referrers.slice(0, 8)) console.log(`    ${r.referrer.padEnd(28)} ${r.count} (${r.uniques} unique)`);
}
console.log("  website visits: Cloudflare dashboard → styky Pages project → Metrics / Web Analytics");
console.log("  Store installs: Partner Center → Styky → Analytics → Acquisitions");
console.log("  sales: Lemon Squeezy dashboard → Orders");

if (process.argv.includes("--log")) {
  const file = "private/stats.csv";
  if (!existsSync(file)) appendFileSync(file, "date,stars,views14d,uniques14d,downloads\n");
  appendFileSync(file, `${today},${repo.stargazers_count},${views?.count ?? ""},${views?.uniques ?? ""},${total}\n`);
  console.log(`  appended to ${file}`);
}
