// Dev helper: screenshot a Styky webview by window label over CDP (port 9222).
//   node scripts/cdpshot.mjs <label> <out.png> ["js to run first"]
const [, , want, out, pre] = process.argv;
import { writeFileSync } from "node:fs";
const targets = (await (await fetch("http://127.0.0.1:9222/json")).json()).filter((t) => t.type === "page");
for (const t of targets) {
  const ws = new WebSocket(t.webSocketDebuggerUrl);
  await new Promise((r) => (ws.onopen = r));
  const call = (method, params) =>
    new Promise((res) => {
      const id = Math.floor(Math.random() * 1e9);
      const h = (m) => {
        const d = JSON.parse(m.data);
        if (d.id === id) {
          ws.removeEventListener("message", h);
          res(d.result);
        }
      };
      ws.addEventListener("message", h);
      ws.send(JSON.stringify({ id, method, params }));
    });
  const label = (await call("Runtime.evaluate", { expression: "window.__TAURI_INTERNALS__.metadata.currentWindow.label", returnByValue: true }))?.result?.value;
  if (label === want) {
    if (pre) {
      await call("Runtime.evaluate", { expression: pre, awaitPromise: true });
      await new Promise((r) => setTimeout(r, 700));
    }
    const r = await call("Page.captureScreenshot", { format: "png" });
    writeFileSync(out, Buffer.from(r.data, "base64"));
    ws.close();
    process.exit(0);
  }
  ws.close();
}
console.error("no page with label", want);
process.exit(1);
