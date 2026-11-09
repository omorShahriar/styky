import data from "./release.json";

/** Latest release facts, baked in at build time (scripts/release-data.mjs). No request is made in the browser. */
export const release = {
  version: data.version,
  /** null while the repo has fewer than 10 stars, so the hero keeps its "Star on GitHub" wording. */
  stars: data.stars >= 10 ? data.stars : null,
} as const;
