export async function writeText(text: string): Promise<void> {
  try {
    await navigator.clipboard.writeText(text);
  } catch {
    /* clipboard unavailable (insecure context or denied): the copy chip still animates */
  }
}
