window.setVal = (el, v) => { const p = el.tagName === "TEXTAREA" ? HTMLTextAreaElement.prototype : HTMLInputElement.prototype; Object.getOwnPropertyDescriptor(p, "value").set.call(el, v); el.dispatchEvent(new Event("input", { bubbles: true })); };
window.sleep = (ms) => new Promise((r) => setTimeout(r, ms));
window.items = () => window.__TAURI_INTERNALS__.invoke("list_items");
"ok"
