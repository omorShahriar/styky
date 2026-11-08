import { useEffect, useRef, useState, type ReactNode } from "react";
import { config } from "../config";

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

/** Latest installer asset and star count, read once from the GitHub API (cached for the session). */
export function useRelease() {
  const [state, setState] = useState<{ url: string; version: string; stars: number | null }>({
    url: config.downloadUrl,
    version: config.version,
    stars: null,
  });
  useEffect(() => {
    const ctl = new AbortController();
    const get = async <T,>(path: string): Promise<T | null> => {
      const key = `styky-gh:${path}`;
      try {
        const hit = sessionStorage.getItem(key);
        if (hit) return JSON.parse(hit) as T;
      } catch {
        /* storage unavailable */
      }
      try {
        const r = await fetch(`https://api.github.com/repos/${config.repo}${path}`, {
          signal: ctl.signal,
          headers: { Accept: "application/vnd.github+json" },
        });
        if (!r.ok) return null;
        const j = (await r.json()) as T;
        try {
          sessionStorage.setItem(key, JSON.stringify(j));
        } catch {
          /* ignore */
        }
        return j;
      } catch {
        return null;
      }
    };
    void Promise.all([
      get<{ stargazers_count: number }>(""),
      get<{ tag_name: string; assets: { name: string; browser_download_url: string }[] }>("/releases/latest"),
    ]).then(([repo, rel]) => {
      const asset = rel?.assets.find((a) => /x64-setup\.exe$/i.test(a.name));
      setState((s) => ({
        url: asset?.browser_download_url ?? s.url,
        version: rel?.tag_name ? rel.tag_name.replace(/^v/, "") : s.version,
        stars: repo?.stargazers_count ?? null,
      }));
    });
    return () => ctl.abort();
  }, []);
  return state;
}
