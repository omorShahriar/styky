// Dev helper: evaluate JS in a specific Styky webview by window label over CDP (port 9222).
//   node scripts/cdpx.mjs main "document.title"
//   node scripts/cdpx.mjs quickadd "document.querySelector('input').value"
// The window label is read inside the page (window.__TAURI_INTERNALS__.metadata.currentWindow.label).
const [, , want, expr] = process.argv;
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
    const r = await call("Runtime.evaluate", { expression: expr, awaitPromise: true, returnByValue: true });
    console.log(JSON.stringify(r.result?.value ?? r.exceptionDetails ?? r, null, 2));
    ws.close();
    process.exit(0);
  }
  ws.close();
}
console.error("no page with label", want);
process.exit(1);
