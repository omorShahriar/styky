import { useEffect, useState, type ReactNode } from "react";
import { Check, Copy, Plus } from "lucide-react";
import { config } from "../config";
import { Footer, LegalNav } from "./Shell";
import { StoreLink } from "./util";

const BASE = import.meta.env.BASE_URL;

/**
 * The /mcp page. Everything here renders to static HTML (react-dom/server): no browser globals at module scope, no
 * lazy content. Facts come from src-tauri/src/cli/mcp.rs and src/app/ConnectAI.tsx; if those change, change this file.
 */

/** Terminal-style block. The text is always in the HTML; the Copy button is added after mount, so without JS it is just selectable text. */
function Code({ code, label }: { code: string; label: string }) {
  const [js, setJs] = useState(false);
  const [done, setDone] = useState(false);
  useEffect(() => setJs(true), []);
  const copy = async () => {
    try {
      await navigator.clipboard.writeText(code);
      setDone(true);
      setTimeout(() => setDone(false), 1600);
    } catch {
      /* clipboard blocked: the text is still selectable */
    }
  };
  return (
    <div className="x-code">
      <pre className="x-term" aria-label={label} tabIndex={0}>
        {code}
      </pre>
      {js && (
        <button className="x-copy" onClick={copy} aria-label={`Copy: ${label}`}>
          {done ? <Check size={14} strokeWidth={2.25} /> : <Copy size={14} strokeWidth={2} />}
          {done ? "Copied" : "Copy"}
        </button>
      )}
    </div>
  );
}

// ------------------------------------------------------------------ client snippets (same text as Settings, Connect AI)

const CLAUDE_CODE = "claude mcp add styky -- styky mcp";

const CLAUDE_DESKTOP = `{
  "mcpServers": {
    "styky": {
      "command": "styky",
      "args": ["mcp"]
    }
  }
}`;

const OTHER_CLIENTS = `{
  "styky": { "type": "stdio", "command": "styky", "args": ["mcp"] }
}`;

// ------------------------------------------------------------------ tools (mcp.rs)

interface Param {
  name: string;
  type: ReactNode;
  note: ReactNode;
}
interface Tool {
  name: string;
  does: ReactNode;
  params: Param[];
}

const ID: Param = {
  name: "id",
  type: "string, required",
  note: "The 8-character shortId from list or add. Any unique prefix of the full id works.",
};
const WHEN =
  "Natural language (“tomorrow 5pm”, “fri 10am”), a short duration (“30m”, “2h”, “1d”) or an ISO date-time (“2026-03-12 17:00” in local time, or RFC 3339).";
const PRIORITY = "low, normal, urgent or critical";
const REPEAT = "none, daily, weekdays, weekly or monthly";

const TOOLS: Tool[] = [
  {
    name: "styky_add",
    does: (
      <>
        Add a reminder. Pass <code>text</code>, a natural-language line such as{" "}
        <code>Release v2.3 fri 5pm #api !!</code>, or explicit fields, or both. Explicit fields override what the text
        says. Needs <code>text</code> or <code>title</code>.
      </>
    ),
    params: [
      { name: "text", type: "string", note: "A quick-add line. Dates and times, #tag, @branch, URLs, !! for urgent and every day|weekday|week|month are parsed." },
      { name: "title", type: "string", note: "The reminder title. Overrides the title parsed from text." },
      { name: "due", type: "string", note: WHEN },
      { name: "tag", type: "string", note: "Tag or project, without the #." },
      { name: "priority", type: "enum", note: PRIORITY },
      { name: "repeat", type: "enum", note: REPEAT },
      { name: "notes", type: "string", note: "Markdown notes." },
      { name: "link", type: "string", note: "A URL to attach." },
      { name: "branch", type: "string", note: "A git branch or repo to attach." },
    ],
  },
  {
    name: "styky_list",
    does: (
      <>
        List reminders, soonest first. Returns id, shortId, title, due (ISO 8601, local time), priority, tag, status and
        whether it is overdue. Notes are left out; use <code>styky_get</code> for them.
      </>
    ),
    params: [
      { name: "due", type: "enum", note: "today, overdue or any. Overdue means past its due or snooze time and still open." },
      { name: "tag", type: "string", note: "Only reminders with this tag." },
      { name: "status", type: "enum", note: "open, done or all. Defaults to open." },
      { name: "limit", type: "integer", note: "1 to 1000." },
    ],
  },
  {
    name: "styky_get",
    does: "Get one reminder with all its fields, including notes.",
    params: [ID],
  },
  {
    name: "styky_complete",
    does: "Mark a reminder done. A repeating reminder rolls forward to its next occurrence instead.",
    params: [ID],
  },
  {
    name: "styky_snooze",
    does: "Snooze a reminder. It notifies again at the time you give.",
    params: [ID, { name: "until", type: "string, required", note: WHEN }],
  },
  {
    name: "styky_update",
    does: (
      <>
        Change fields of a reminder. Only the fields you pass change. Pass <code>due: null</code> to clear the due time
        and <code>tag: null</code> to clear the tag.
      </>
    ),
    params: [
      ID,
      { name: "title", type: "string", note: "New title." },
      { name: "due", type: "string or null", note: <>New due time, or null to clear it. {WHEN}</> },
      { name: "tag", type: "string or null", note: "New tag, or null to clear it." },
      { name: "priority", type: "enum", note: PRIORITY },
      { name: "repeat", type: "enum", note: REPEAT },
      { name: "notes", type: "string", note: "Markdown notes." },
      { name: "link", type: "string or null", note: "A URL, or null to clear it." },
      { name: "branch", type: "string or null", note: "A git branch or repo, or null to clear it." },
    ],
  },
  {
    name: "styky_delete",
    does: "Permanently delete a reminder. The tool description tells the assistant to prefer styky_complete unless you asked to delete.",
    params: [ID],
  },
];

// ------------------------------------------------------------------ examples (grammar: src-tauri/src/nlp.rs, nlp_repeat.rs)

const EXAMPLES: { ask: string; call: ReactNode; makes: string }[] = [
  {
    ask: "Remind me to deploy the api on Friday at 5pm, tag it infra.",
    call: <code>styky_add text: &quot;Deploy api fri 5pm #infra&quot;</code>,
    makes: "A reminder “Deploy api”, due Friday 17:00, tagged infra.",
  },
  {
    ask: "Rotate the prod keys Friday at 10, it's urgent.",
    call: <code>styky_add text: &quot;Rotate prod keys fri 10am !!&quot;</code>,
    makes: "An urgent reminder due Friday 10:00.",
  },
  {
    ask: "Every weekday at 9:30, remind me to check the CI dashboard.",
    call: <code>styky_add text: &quot;Check CI dashboard every weekday 9:30am&quot;</code>,
    makes: "A repeating reminder on weekdays at 09:30, starting at the next one.",
  },
  {
    ask: "In two hours, remind me to check the deploy for feat/login, and attach the PR link.",
    call: <code>styky_add text: &quot;Check deploy in 2h @feat/login https://github.com/acme/api/pull/42&quot;</code>,
    makes: "A reminder due in two hours with the branch feat/login and the PR link attached.",
  },
  {
    ask: "What's overdue?",
    call: <code>styky_list due: &quot;overdue&quot;</code>,
    makes: "Nothing new. The assistant reads back the open reminders that are past due.",
  },
  {
    ask: "Snooze the keys reminder until tomorrow 9am.",
    call: <code>styky_snooze id, until: &quot;tomorrow 9am&quot;</code>,
    makes: "The same reminder, notifying again tomorrow at 09:00. The assistant finds the id with styky_list.",
  },
];

// ------------------------------------------------------------------ questions

const FAQ: { q: string; a: ReactNode }[] = [
  {
    q: "Does the Styky app have to be running?",
    a: (
      <p>
        Yes. <code>styky mcp</code> talks to the running app, which owns the reminders and fires the notifications. With
        Styky closed, every tool call fails with &ldquo;Styky isn&rsquo;t running&rdquo;. Styky starts with Windows by
        default.
      </p>
    ),
  },
  {
    q: "Does it work with ChatGPT?",
    a: (
      <p>
        Not directly. ChatGPT only connects to remote HTTPS MCP servers, and <code>styky mcp</code> is a local one.
        Claude Desktop, Claude Code and other clients that launch a local command work today.
      </p>
    ),
  },
  {
    q: "Is it free?",
    a: (
      <p>
        Yes. All seven tools are free, and Styky itself is free and open source under the MIT license. Read the code on{" "}
        <a href={config.repoUrl} rel="noopener">
          GitHub
        </a>
        .
      </p>
    ),
  },
  {
    q: "Does the Microsoft Store version include it?",
    a: (
      <p>
        Yes, from v0.2.1. The Store version adds the <code>styky</code> command through an app execution alias, so it
        works in a new terminal right after install. The GitHub installer puts the same command on your PATH instead.
      </p>
    ),
  },
  {
    q: "What does “Styky isn't running” mean?",
    a: (
      <>
        <p>
          The MCP server started, but it could not find the Styky app. The full message is &ldquo;Styky isn&rsquo;t
          running. Start it from the Start menu, then try again.&rdquo; The assistant sees it as a tool error.
        </p>
        <p>Start Styky and ask again. Nothing needs to be reconfigured.</p>
      </>
    ),
  },
  {
    q: "Can the assistant delete my reminders?",
    a: (
      <p>
        There is a <code>styky_delete</code> tool, and it deletes for good. Its description tells the assistant to prefer{" "}
        <code>styky_complete</code> unless you asked for a delete. Whether your client asks you before it runs a tool is a
        setting of that client.
      </p>
    ),
  },
  {
    q: "How do I remove it from a client?",
    a: (
      <>
        <p>
          Claude Code: run <code>claude mcp remove styky</code>. Claude Desktop: delete the <code>styky</code> entry from{" "}
          <code>claude_desktop_config.json</code> and restart Claude. Other clients: delete the same entry from the
          client&rsquo;s MCP server list.
        </p>
        <p>Your reminders stay in Styky. Removing the connection only stops the assistant from reaching them.</p>
      </>
    ),
  },
];

// ------------------------------------------------------------------ page

const TOC: [string, string][] = [
  ["need", "What you need"],
  ["setup", "Set it up"],
  ["tools", "Tools"],
  ["examples", "Things to ask"],
  ["how", "How it works"],
  ["faq", "Questions"],
];

export function McpPage() {
  return (
    <>
      <LegalNav />
      <main className="x-legal x-mcp">
        <div className="x-wrap">
          <nav className="x-crumbs" aria-label="Breadcrumb">
            <a href={BASE}>Home</a>
            <span aria-hidden>/</span>
            <span aria-current="page">MCP server</span>
          </nav>

          <h1>A reminder MCP server for Windows</h1>
          <p className="x-mcp-lead">
            Ask Claude, or any MCP client, to remember something. The assistant adds the reminder through Styky, it shows
            up in the Styky panel on your desktop, and a native Windows notification with Done and Snooze fires when it is
            due.
          </p>

          <nav className="x-toc" aria-label="On this page">
            {TOC.map(([id, label]) => (
              <a key={id} href={`#${id}`}>
                {label}
              </a>
            ))}
          </nav>

          <section id="need" aria-labelledby="need-h">
            <h2 id="need-h">What you need</h2>
            <ul>
              <li>Windows 10 (version 1809 or newer) or Windows 11, 64-bit.</li>
              <li>
                Styky from the <StoreLink>Microsoft Store</StoreLink> (v0.2.1 or newer), or v0.2.0 or newer from the{" "}
                <a href={config.downloadUrl} rel="noopener">
                  installer on GitHub Releases
                </a>
                . Both give you the <code>styky</code> command in any new terminal.
              </li>
              <li>The Styky app running. It starts with Windows by default.</li>
            </ul>
            <p>
              To check the install, open a new terminal and run <code>styky ls --today</code>. It lists today&rsquo;s
              reminders, or says that Styky isn&rsquo;t running.
            </p>
          </section>

          <section id="setup" aria-labelledby="setup-h">
            <h2 id="setup-h">Set it up</h2>
            <p>
              The server is the <code>styky</code> command with the argument <code>mcp</code>. It speaks MCP over stdio,
              so any client that can launch a local command can use it. These are the same snippets as Settings, Connect AI
              in the app.
            </p>

            <h3 id="setup-claude-code">Claude Code</h3>
            <p>Run once in a terminal.</p>
            <Code code={CLAUDE_CODE} label="Add the Styky MCP server to Claude Code" />

            <h3 id="setup-claude-desktop">Claude Desktop</h3>
            <p>
              Add this to <code>claude_desktop_config.json</code> (Settings, Developer, Edit Config), then restart Claude.
            </p>
            <Code code={CLAUDE_DESKTOP} label="Claude Desktop MCP configuration" />

            <h3 id="setup-other">Cursor, VS Code, Codex and other clients</h3>
            <p>
              Any client that can launch a local stdio MCP server works: command <code>styky</code>, argument{" "}
              <code>mcp</code>. Add this entry to the client&rsquo;s MCP server list, in whatever format that client uses.
            </p>
            <Code code={OTHER_CLIENTS} label="Generic stdio MCP server entry for Styky" />
          </section>

          <section id="tools" aria-labelledby="tools-h">
            <h2 id="tools-h">Tools</h2>
            <p>
              Seven tools, all free. They are the only things the server exposes: no resources, no prompts. Times are
              read in your local time zone.
            </p>
            {TOOLS.map((t) => (
              <div className="x-tool-doc" key={t.name} id={t.name.replace("_", "-")}>
                <h3>
                  <code>{t.name}</code>
                </h3>
                <p>{t.does}</p>
                <div className="x-table-wrap">
                  <table className="x-params">
                    <thead>
                      <tr>
                        <th scope="col">Parameter</th>
                        <th scope="col">Type</th>
                        <th scope="col">Notes</th>
                      </tr>
                    </thead>
                    <tbody>
                      {t.params.map((p) => (
                        <tr key={p.name}>
                          <th scope="row">
                            <code>{p.name}</code>
                          </th>
                          <td>{p.type}</td>
                          <td>{p.note}</td>
                        </tr>
                      ))}
                    </tbody>
                  </table>
                </div>
              </div>
            ))}
            <p>
              Repeats are limited to daily, weekdays, weekly and monthly. A request for anything else is refused with a
              message the assistant can read.
            </p>
          </section>

          <section id="examples" aria-labelledby="examples-h">
            <h2 id="examples-h">Things to ask</h2>
            <p>Plain sentences work. The assistant turns them into a tool call like the ones below.</p>
            <ul className="x-asks">
              {EXAMPLES.map((e) => (
                <li key={e.ask}>
                  <p className="x-ask">&ldquo;{e.ask}&rdquo;</p>
                  <p>{e.call}</p>
                  <p>{e.makes}</p>
                </li>
              ))}
            </ul>
          </section>

          <section id="how" aria-labelledby="how-h">
            <h2 id="how-h">How it works</h2>
            <ul>
              <li>
                <code>styky mcp</code> is a stdio server. Your client starts it and talks to it over stdin and stdout. It
                speaks MCP protocol versions 2025-11-25, 2025-06-18, 2025-03-26 and 2024-11-05.
              </li>
              <li>
                The server does not touch your reminders itself. It forwards each call to the running Styky app over a
                Windows named pipe that only your Windows account can open.
              </li>
              <li>Nothing listens on the network.</li>
              <li>The app is the only process that writes the SQLite file. Your reminders stay on your PC.</li>
              <li>
                Your assistant, your call: whatever it reads from Styky goes to that assistant&rsquo;s provider. Connect
                only assistants you trust.
              </li>
            </ul>
            <p>
              Styky itself has no telemetry. The full statement is in the <a href={`${BASE}privacy`}>privacy policy</a>.
            </p>
          </section>

          <section id="faq" aria-labelledby="faq-h">
            <h2 id="faq-h">Questions</h2>
            <div className="x-faq x-mcp-faq">
              {FAQ.map((f) => (
                <details key={f.q}>
                  <summary>
                    {f.q}
                    <Plus size={18} strokeWidth={2} aria-hidden />
                  </summary>
                  <div className="x-ans">{f.a}</div>
                </details>
              ))}
            </div>
          </section>

          <section id="download" className="x-mcp-cta" aria-labelledby="download-h">
            <h2 id="download-h">Get Styky</h2>
            <p>Free, open source, MIT licensed. Windows 10 and 11.</p>
            <div className="x-mcp-cta-row">
              <StoreLink className="x-btn primary">Get it from Microsoft Store</StoreLink>
              <a className="x-btn secondary" href={config.downloadUrl} rel="noopener">
                Installer from GitHub
              </a>
            </div>
          </section>
        </div>
      </main>
      <Footer />
    </>
  );
}
