import { StrictMode } from "react";
import { createRoot, hydrateRoot } from "react-dom/client";
import "./styles/site.css";
import "./styles/mcp.css";
import { McpPage } from "./sections/McpPage";

const root = document.getElementById("root") as HTMLElement;
const page = (
  <StrictMode>
    <McpPage />
  </StrictMode>
);

// `pnpm build` prerenders the page into dist/mcp.html (scripts/prerender.mjs): hydrate it. The dev server serves an empty root.
if (root.hasChildNodes()) hydrateRoot(root, page);
else createRoot(root).render(page);
