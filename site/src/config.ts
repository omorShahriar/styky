/**
 * Site configuration. Edit this file (or set the VITE_* env vars at build time).
 * See site/README.md for how to wire up Lemon Squeezy and a custom domain.
 */

/** Marker value: while the checkout URL contains it, the Buy button shows "Coming soon". */
export const CHECKOUT_PLACEHOLDER = "REPLACE_ME";

export const config = {
  repo: "omorShahriar/styky",
  repoUrl: "https://github.com/omorShahriar/styky",
  releasesUrl: "https://github.com/omorShahriar/styky/releases/latest",
  /** Installer asset of the current release. Refined at runtime from the GitHub API if the version moved on. */
  downloadUrl: "https://github.com/omorShahriar/styky/releases/latest/download/Styky_0.1.0_x64-setup.exe",
  version: "0.1.0",
  licenseUrl: "https://github.com/omorShahriar/styky/blob/main/LICENSE",
  privacyUrl: "https://github.com/omorShahriar/styky/blob/main/docs/PRIVACY.md",
  issuesUrl: "https://github.com/omorShahriar/styky/issues",
  author: "Omor Shahriar",
  authorUrl: "https://github.com/omorShahriar",

  /**
   * Lemon Squeezy checkout URL for Styky Pro, e.g. "https://YOURSTORE.lemonsqueezy.com/buy/<uuid>".
   * Leave the placeholder until the product exists; the Buy button then reads "Coming soon".
   */
  checkoutUrl:
    (import.meta.env.VITE_LS_CHECKOUT_URL as string | undefined) ||
    `https://YOURSTORE.lemonsqueezy.com/buy/${CHECKOUT_PLACEHOLDER}`,

  /** Early-bird pre-order price, and the price once Pro v1 ships (USD). */
  priceEarly: 4.99,
  priceRegular: 6.99,
} as const;

export const checkoutReady = !config.checkoutUrl.includes(CHECKOUT_PLACEHOLDER);

export const money = (n: number) => `$${n.toFixed(2)}`;
