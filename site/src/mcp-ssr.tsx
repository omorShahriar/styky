import { renderToString } from "react-dom/server";
import { McpPage } from "./sections/McpPage";

/** Build-time only: scripts/prerender.mjs loads this through Vite and writes the markup into dist/mcp.html. */
export function render(): string {
  return renderToString(<McpPage />).replace(/^(<link rel="preload"[^>]*\/>)+/, "");
}
