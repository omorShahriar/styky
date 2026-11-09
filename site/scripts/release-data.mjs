// Refreshes src/release.json (latest release tag and star count) from the GitHub API at BUILD time.
// The browser never calls GitHub. If the API is unreachable, rate-limited (60 req/h per IP without a token)
// or answers anything unexpected, the committed file is kept and the build carries on: this script never fails.
//
//   GITHUB_TOKEN   optional, raises the rate limit in CI (never written anywhere)
//   GITHUB_API_URL optional override, e.g. http://127.0.0.1:9 to simulate an unreachable API in a test

import { readFileSync, writeFileSync } from "node:fs";
import { fileURLToPath } from "node:url";

const file = fileURLToPath(new URL("../src/release.json", import.meta.url));
const repo = "omorShahriar/styky";
const api = (process.env.GITHUB_API_URL || "https://api.github.com").replace(/\/$/, "");
const headers = { Accept: "application/vnd.github+json", "User-Agent": "styky-site-build" };
if (process.env.GITHUB_TOKEN) headers.Authorization = `Bearer ${process.env.GITHUB_TOKEN}`;

async function get(path) {
  const r = await fetch(`${api}/repos/${repo}${path}`, { headers, signal: AbortSignal.timeout(8000) });
  if (!r.ok) throw new Error(`${path || "/"} -> HTTP ${r.status}`);
  return r.json();
}

const current = JSON.parse(readFileSync(file, "utf8"));
try {
  const [info, rel] = await Promise.all([get(""), get("/releases/latest")]);
  const next = {
    version: String(rel.tag_name).replace(/^v/, ""),
    stars: Number(info.stargazers_count),
    fetched: new Date().toISOString().slice(0, 10),
  };
  if (!/^\d+\.\d+\.\d+/.test(next.version) || !Number.isFinite(next.stars)) throw new Error("unexpected response");
  writeFileSync(file, JSON.stringify(next, null, 2) + "\n");
  console.log(`release-data: v${next.version}, ${next.stars} stars (fetched ${next.fetched})`);
} catch (e) {
  console.warn(`release-data: GitHub API unavailable (${e.message}); keeping the committed src/release.json (v${current.version}, ${current.stars} stars)`);
}
