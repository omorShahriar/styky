import { StrictMode } from "react";
import { prerender } from "react-dom/static";
import App from "./App";

/**
 * Build-time only: scripts/prerender.mjs loads this through Vite and writes the markup into dist/index.html.
 * `prerender` waits for lazy components, so the Grammar section is in the HTML. The demo is not: it is mounted by an
 * effect after hydration (see DemoSlot in sections/Chrome.tsx), so the server render only contains its skeleton.
 */
export async function renderHome(): Promise<string> {
  const { prelude } = await prerender(
    <StrictMode>
      <App />
    </StrictMode>,
    // Inline every finished Suspense boundary (Grammar) instead of streaming it into a hidden <div> plus a swap script.
    { progressiveChunkSize: Infinity },
  );
  const chunks: string[] = [];
  const decoder = new TextDecoder();
  for await (const c of prelude as unknown as AsyncIterable<Uint8Array>) chunks.push(decoder.decode(c, { stream: true }));
  chunks.push(decoder.decode());
  // React hoists an image preload (the nav logo) into the markup; the client does not render it, so drop it before hydration.
  return chunks.join("").replace(/^(<link rel="preload"[^>]*\/>)+/, "");
}
