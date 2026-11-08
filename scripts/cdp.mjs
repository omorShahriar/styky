// Dev helper: drive the running Styky webview over the Chrome DevTools Protocol.
// Start the app with WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS=--remote-debugging-port=9222, then:
//   node scripts/cdp.mjs eval "document.title"
//   node scripts/cdp.mjs shot out.png
//   node scripts/cdp.mjs key ArrowDown        (dispatch a keydown on window)
import { writeFileSync } from "node:fs";

const [, , cmd, arg] = process.argv;
const targets = await (await fetch("http://127.0.0.1:9222/json")).json();
const page = targets.find((t) => t.type === "page" && /localhost:1420|tauri\.localhost/.test(t.url));
if (!page) throw new Error("no Styky page target found");

const ws = new WebSocket(page.webSocketDebuggerUrl);
await new Promise((r) => (ws.onopen = r));
let id = 0;
const pending = new Map();
ws.onmessage = (m) => {
  const d = JSON.parse(m.data);
  if (d.id && pending.has(d.id)) pending.get(d.id)(d);
};
const send = (method, params = {}) =>
  new Promise((res) => {
    const i = ++id;
    pending.set(i, res);
    ws.send(JSON.stringify({ id: i, method, params }));
  });

if (cmd === "eval") {
  const r = await send("Runtime.evaluate", { expression: arg, awaitPromise: true, returnByValue: true });
  console.log(JSON.stringify(r.result?.result?.value ?? r.result?.exceptionDetails ?? r.result, null, 2));
} else if (cmd === "shot") {
  const r = await send("Page.captureScreenshot", { format: "png" });
  writeFileSync(arg, Buffer.from(r.result.data, "base64"));
  console.log("saved", arg);
} else if (cmd === "key") {
  for (const type of ["keyDown", "keyUp"]) {
    await send("Input.dispatchKeyEvent", { type, key: arg, code: arg, windowsVirtualKeyCode: ({ ArrowDown: 40, ArrowUp: 38, Enter: 13, Escape: 27, Delete: 46, " ": 32 })[arg] ?? arg.toUpperCase().charCodeAt(0), text: arg.length === 1 ? arg : undefined });
  }
  console.log("key", arg);
}
ws.close();
