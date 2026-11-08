import { tagHue } from "../lib/tags";

export function TagChip({ tag }: { tag: string }) {
  return (
    <span className="chip tag-chip" style={{ ["--hue" as string]: tagHue(tag) }}>
      {tag}
    </span>
  );
}
