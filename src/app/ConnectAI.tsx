import { useEffect, useState } from "react";
import { api } from "../lib/api";
import { ProPill } from "../components/ProPill";
import { usePro } from "../lib/pro";

interface Snippet {
  id: string;
  title: string;
  hint: string;
  code: string;
}

const SNIPPETS: Snippet[] = [
  {
    id: "claude-desktop",
    title: "Claude Desktop",
    hint: "Add to claude_desktop_config.json (Settings → Developer → Edit Config), then restart Claude.",
    code: `{
  "mcpServers": {
    "styky": {
      "command": "styky",
      "args": ["mcp"]
    }
  }
}`,
  },
  {
    id: "claude-code",
    title: "Claude Code",
    hint: "Run once in a terminal.",
    code: "claude mcp add styky -- styky mcp",
  },
  {
    id: "other",
    title: "Other MCP clients (Cursor, VS Code, Codex…)",
    hint: "Any client that can launch a local stdio MCP server: command `styky`, argument `mcp`. ChatGPT only talks to remote (HTTPS) MCP servers today, so it cannot use a local one directly.",
    code: `{
  "styky": { "type": "stdio", "command": "styky", "args": ["mcp"] }
}`,
  },
];

async function copy(text: string) {
  try {
    await navigator.clipboard.writeText(text);
    return true;
  } catch {
    return false;
  }
}

/** Settings → Connect AI: copy-paste MCP setup. The server is the `styky` CLI that ships with the app. */
export function ConnectAI() {
  const pro = usePro();
  const [copied, setCopied] = useState<string | null>(null);
  const [failed, setFailed] = useState(false);
  const [apiError, setApiError] = useState<string | null>(null);

  useEffect(() => {
    const check = () => void api.apiStatus().then(setApiError).catch(() => {});
    check();
    window.addEventListener("focus", check);
    return () => window.removeEventListener("focus", check);
  }, []);

  const onCopy = async (s: Snippet) => {
    const ok = await copy(s.code);
    setFailed(!ok);
    setCopied(ok ? s.id : null);
    window.setTimeout(() => setCopied((c) => (c === s.id ? null : c)), 1800);
  };

  return (
    <section className="group" id="connect-ai">
      <h2>Connect AI</h2>
      <div className="card">
        <div className="ai-body">
          <p className="ai-lede">
            Let Claude (or any MCP client) add and read your reminders: &ldquo;remind me to rotate the keys Friday&rdquo;. The connector is the{" "}
            <code>styky</code> command that installs with the app; Styky must be running.
          </p>
          {apiError && (
            <p className="inline-error" role="alert">
              Local API unavailable: {apiError}. The styky command and MCP connectors cannot reach Styky until this is fixed (another program may be using the Styky pipe; restart Styky or that program).
            </p>
          )}
          {SNIPPETS.map((s) => (
            <div className="snippet" key={s.id}>
              <div className="snippet-head">
                <strong>{s.title}</strong>
                <button className="btn" onClick={() => void onCopy(s)}>
                  {copied === s.id ? "Copied" : "Copy"}
                </button>
              </div>
              <pre>{s.code}</pre>
              <p className="muted small">{s.hint}</p>
            </div>
          ))}
          {failed && <p className="inline-error">Couldn&rsquo;t copy automatically; select the text and press Ctrl+C.</p>}
          <p className="muted small ai-tier">
            Adding and listing reminders is free.{" "}
            {pro ? "Completing, snoozing, editing and deleting are unlocked." : <>Completing, snoozing, editing and deleting need Styky Pro <ProPill /></>}
          </p>
        </div>
      </div>
    </section>
  );
}
