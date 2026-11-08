import { useState } from "react";
import { TypeBar, prefersReducedMotion } from "../demo/TypeBar";
import { Reveal, SectionHead } from "./util";

const EXAMPLES = [
  "Release v2.3 tomorrow 5pm #api !!",
  "Rotate the API keys every month #security",
  "Standup notes every weekday 9:30am",
  "Review PR https://github.com/acme/app/pull/42 @fix/login in 2h",
  "Call mum sunday 6pm !low",
];

const LEGEND: [string, string][] = [
  ["tomorrow 5pm", "any date or time, in plain words"],
  ["#tag", "a project or topic"],
  ["!!", "urgent (!low for low)"],
  ["every week", "day, weekday, week or month"],
  ["@branch", "a branch or repo chip"],
  ["https://…", "a link, kept on the reminder"],
];

export function Grammar() {
  const [i, setI] = useState(0);
  const reduced = prefersReducedMotion();
  return (
    <section className="x-section alt" id="quick-add">
      <div className="x-wrap">
        <SectionHead
          eyebrow="Quick add"
          title="Type it like you say it."
          lead="One line becomes a title, a due time, a tag, a priority and a repeat. The chips show what Styky understood before you press Enter. It is the same parser the app uses."
        />
        <div className="x-gram">
          <Reveal>
            <ul className="x-examples" aria-label="Examples">
              {EXAMPLES.map((e, n) => (
                <li key={e}>
                  <button type="button" className="x-example" aria-pressed={n === i} onClick={() => setI(n)}>
                    {e}
                  </button>
                </li>
              ))}
            </ul>
          </Reveal>
          <Reveal delay={80}>
            <div className="x-gram-stage">
              <div className="x-wallpaper" aria-hidden>
                <i className="x-bloom x-b1" />
                <i className="x-bloom x-b2" />
                <i className="x-bloom x-b3" />
              </div>
              <div className="x-gram-bar">
                <TypeBar phrases={EXAMPLES} index={i} onDone={() => !reduced && setI((n) => (n + 1) % EXAMPLES.length)} />
              </div>
              <div className="x-legend">
                {LEGEND.map(([k, v]) => (
                  <div key={k}>
                    <b>{k}</b>
                    {v}
                  </div>
                ))}
              </div>
            </div>
          </Reveal>
        </div>
      </div>
    </section>
  );
}
