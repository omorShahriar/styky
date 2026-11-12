import { renderToString } from "react-dom/server";
import { LegalPage, type LegalPageName } from "./sections/LegalPages";

/** Build-time only: scripts/prerender.mjs loads this through Vite and writes the markup into dist/<page>.html. */
export function render(name: LegalPageName): string {
  return renderToString(<LegalPage name={name} />).replace(/^(<link rel="preload"[^>]*\/>)+/, "");
}
