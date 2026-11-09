import { useState } from "react";
import { Check, Copy } from "lucide-react";
import { config, proVersion, versionAtLeast } from "../config";
import { Reveal, SectionHead, useRelease } from "./util";

type Tier = "free" | "pro";

function Code({ code, label }: { code: string; label: string }) {
  const [done, setDone] = useState(false);
  const copy = async () => {
    try {
      await navigator.clipboard.writeText(code.replace(/^\$ /gm, ""));
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
      <button className="x-copy" onClick={copy} aria-label={`Copy: ${label}`}>
        {done ? <Check size={14} strokeWidth={2.25} /> : <Copy size={14} strokeWidth={2} />}
        {done ? "Copied" : "Copy"}
      </button>
    </div>
  );
}

function Tag({ tier }: { tier: Tier }) {
  return <span className={`x-tier ${tier}`}>{tier === "pro" ? "Pro" : "Free"}</span>;
}

const CLAUDE_DESKTOP = `{
  "mcpServers": {
    "styky": {
      "command": "styky",
      "args": ["mcp"]
    }
  }
}`;

const HOOK = `#!/bin/sh
# .git/hooks/pre-push   (Git for Windows runs this as-is)
branch=$(git rev-parse --abbrev-ref HEAD)
styky add "Check CI in 15m #ci @$branch" || true
exit 0`;

export function Connect() {
  const { version } = useRelease();
  const shipped = versionAtLeast(version, proVersion);
  const short = proVersion.replace(/\.0$/, "");
  return (
    <section className="x-section alt" id="connect">
      <div className="x-wrap">
        <SectionHead
          eyebrow={`New in v${short}`}
          title="Works with Claude, ChatGPT and your terminal"
          lead={
            <>
              Styky ships a <code>styky</code> command and a local MCP server. The Styky app has to be running; the
              command talks to it over a named pipe that only your Windows account can open.
              {shipped ? "" : ` Included in the v${proVersion} installer.`}
            </>
          }
          center
        />

        <div className="x-snips">
          <Reveal>
            <div className="x-card x-snip">
              <h3>
                Claude Code <Tag tier="free" />
              </h3>
              <p>One command. Add and list are free; complete, snooze, update and delete need Pro.</p>
              <Code code="claude mcp add styky -- styky mcp" label="Add the Styky MCP server to Claude Code" />
            </div>
          </Reveal>

          <Reveal delay={70}>
            <div className="x-card x-snip">
              <h3>
                Claude Desktop <Tag tier="free" />
              </h3>
              <p>
                Paste into <code>claude_desktop_config.json</code> (Settings, Developer, Edit Config) and restart
                Claude. Cursor, VS Code and other clients that launch a local stdio server use the same command.
              </p>
              <Code code={CLAUDE_DESKTOP} label="Claude Desktop MCP configuration" />
            </div>
          </Reveal>

          <Reveal>
            <div className="x-card x-snip">
              <h3>
                Terminal <Tag tier="free" />
              </h3>
              <p>Same grammar as the Ctrl+Alt+N bar: dates, #tags, @branch, !! urgent, repeats.</p>
              <Code
                code={`$ styky add "Deploy api fri 5pm #infra !!"
$ styky ls --today
$ styky ls --overdue --tag infra --json`}
                label="Add and list reminders from a terminal"
              />
              <p className="x-snip-sub">
                <Tag tier="pro" /> <code>!!!</code> is Critical and keeps re-notifying. <code>get</code>,{" "}
                <code>done</code>, <code>snooze</code>, <code>edit</code> and <code>rm</code> are Pro too.
              </p>
              <Code
                code={`$ styky add "Rotate prod keys fri 10am #infra !!!"
$ styky done a1b2c3d4
$ styky snooze a1b2c3d4 "tomorrow 9am"`}
                label="Pro commands: critical add, done, snooze"
              />
            </div>
          </Reveal>

          <Reveal delay={70}>
            <div className="x-card x-snip">
              <h3>
                Git hook <Tag tier="free" />
              </h3>
              <p>
                Git has no client-side post-push hook, so use <code>pre-push</code>: it queues the CI check as you push.
                If the push is rejected, the reminder is still there to dismiss.
              </p>
              <Code code={HOOK} label="A git pre-push hook that adds a Styky reminder" />
            </div>
          </Reveal>
        </div>

        <Reveal delay={60}>
          <div className="x-card x-chat">
            <div>
              <p className="x-eyebrow">MCP tools</p>
              <h3 style={{ fontSize: 26, lineHeight: 1.2, letterSpacing: "-0.025em", fontWeight: 650 }}>
                &ldquo;Remind me to rotate the keys on Friday.&rdquo;
              </h3>
              <p style={{ marginTop: 12, fontSize: 16, color: "var(--label-2)" }}>
                <Tag tier="free" /> <code>styky_add</code>, <code>styky_list</code>
                <br />
                <Tag tier="pro" /> <code>styky_get</code>, <code>styky_complete</code>, <code>styky_snooze</code>,{" "}
                <code>styky_update</code>, <code>styky_delete</code>
              </p>
              <p style={{ marginTop: 12, fontSize: 14.5, color: "var(--label-2)" }}>
                <b>ChatGPT:</b> it only connects to remote HTTPS MCP servers today, so it cannot launch a local one
                like <code>styky mcp</code>. Use the CLI from scripts, and any client that can run a local command. A
                local HTTP endpoint for ChatGPT is on the roadmap, not shipped.
              </p>
            </div>
            <div className="x-chat-box" role="img" aria-label="Illustration: asking an AI assistant to add a Styky reminder">
              <div className="x-bubble me">Remind me to rotate the keys on Friday at 10, it&rsquo;s urgent.</div>
              <div className="x-tool">styky_add &#123; text: &quot;Rotate the keys fri 10am !!&quot; &#125;</div>
              <div className="x-bubble ai">Added &ldquo;Rotate the keys&rdquo; for Friday at 10:00, marked urgent.</div>
              <p className="x-caption-sm">
                Illustration. Whatever an assistant reads goes to that assistant, so connect only ones you trust.{" "}
                <a href={`${config.repoUrl}#use-it-from-claude-chatgpt-and-the-terminal`} rel="noopener">
                  Full setup and tool list
                </a>
                .
              </p>
            </div>
          </div>
        </Reveal>
      </div>
    </section>
  );
}
