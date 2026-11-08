const TASK_LINE = /^(\s*(?:[-*+]|\d+[.)])\s+\[)([ xX])(\])/;

/**
 * Flip the task-list checkbox on `line` (1-based) of `source`, rewriting only
 * that line. Returns `source` unchanged if the line is not a task item.
 */
export function toggleTaskLine(source: string, line: number): string {
  const lines = source.split("\n");
  const idx = line - 1;
  const text = lines[idx];
  if (text === undefined) return source;
  const m = TASK_LINE.exec(text);
  if (!m) return source;
  const next = m[2] === " " ? "x" : " ";
  lines[idx] = text.slice(0, m[1].length) + next + text.slice(m[1].length + 1);
  return lines.join("\n");
}

/** Is the task on `line` (1-based) ticked? `null` if it is not a task line. */
export function isTaskChecked(source: string, line: number): boolean | null {
  const text = source.split("\n")[line - 1];
  const m = text === undefined ? null : TASK_LINE.exec(text);
  return m ? m[2] !== " " : null;
}
