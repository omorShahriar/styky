import { useEffect, useRef, useState, type ReactNode } from "react";
import { config } from "../config";

/** Campaign paths (public/_redirects) that tag outbound Store links, as the inline script in index.html does for data-store links. */
const CHANNELS = ["sp", "w11", "mcp", "hn", "cai", "registry", "readme", "pro"];

/** The Store listing URL with `cid` set to the page's channel ("site-hn" on /hn, else "site"), for Partner Center's campaign report. */
function storeHref(path: string): string {
  const p = path.replace(/\/$/, "").slice(1);
  const u = new URL(config.storeUrl);
  u.searchParams.set("cid", CHANNELS.includes(p) ? `site-${p}` : "site");
  return u.toString();
}

/**
 * A link to the Microsoft Store listing. The server render uses the untagged channel ("site"); after hydration the
 * href picks up the page's channel, so the prerendered HTML and the first client render match.
 */
export function StoreLink({ className, children }: { className?: string; children: ReactNode }) {
  const [href, setHref] = useState(() => storeHref("/"));
  useEffect(() => setHref(storeHref(location.pathname)), []);
  return (
    <a className={className} href={href} rel="noopener">
      {children}
    </a>
  );
}

/** Fade/slide a block in once, when it first scrolls into view. */
export function Reveal({ children, delay = 0, className = "" }: { children: ReactNode; delay?: number; className?: string }) {
  const ref = useRef<HTMLDivElement>(null);
  const [shown, setShown] = useState(false);
  useEffect(() => {
    const el = ref.current;
    if (!el) return;
    if (typeof IntersectionObserver === "undefined") return setShown(true);
    const io = new IntersectionObserver(
      ([e]) => {
        if (e.isIntersecting) {
          setShown(true);
          io.disconnect();
        }
      },
      { rootMargin: "0px 0px -8% 0px" },
    );
    io.observe(el);
    return () => io.disconnect();
  }, []);
  return (
    <div ref={ref} className={`x-reveal${shown ? " in" : ""} ${className}`} style={{ ["--d" as string]: `${delay}ms` }}>
      {children}
    </div>
  );
}

export function SectionHead({
  eyebrow,
  title,
  lead,
  center,
}: {
  eyebrow: string;
  title: ReactNode;
  lead?: ReactNode;
  center?: boolean;
}) {
  return (
    <Reveal className={`x-head${center ? " center" : ""}`}>
      <p className="x-eyebrow">{eyebrow}</p>
      <h2 className="x-h2">{title}</h2>
      {lead && <p className="x-lead">{lead}</p>}
    </Reveal>
  );
}
