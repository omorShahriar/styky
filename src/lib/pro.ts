import { useEffect, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { api } from "./api";

export interface LicenseStatus {
  pro: boolean;
  /** none | active | grace-expired | dev */
  state: "none" | "active" | "grace-expired" | "dev";
  keyHint: string | null;
  customer: string | null;
  validatedAt: number | null;
  buyUrl: string;
}

/** Live Pro entitlement (re-read on every `pro-changed`). `null` until the first answer. */
export function useLicense(): LicenseStatus | null {
  const [status, setStatus] = useState<LicenseStatus | null>(null);
  useEffect(() => {
    let dead = false;
    const load = () => void api.licenseStatus().then((s) => !dead && setStatus(s)).catch(() => {});
    load();
    const off = listen("pro-changed", load);
    return () => {
      dead = true;
      void off.then((u) => u());
    };
  }, []);
  return status;
}

export function usePro(): boolean {
  return useLicense()?.pro ?? false;
}
