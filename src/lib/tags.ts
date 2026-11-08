/** Stable hue (0-359) for a tag name; the same tag always gets the same colour. */
export function tagHue(tag: string): number {
  let h = 2166136261; // FNV-1a
  for (const ch of tag.toLowerCase()) {
    h ^= ch.codePointAt(0) ?? 0;
    h = Math.imul(h, 16777619);
  }
  // A hand-picked set of hues that stay legible in both themes.
  const hues = [210, 262, 330, 8, 28, 150, 180, 238, 292, 350];
  return hues[(h >>> 0) % hues.length];
}
