/** Links in demo reminders open in a new tab (http/https only). */
export async function openUrl(url: string | URL): Promise<void> {
  const href = String(url);
  if (/^https?:\/\//i.test(href)) window.open(href, "_blank", "noopener,noreferrer");
}
