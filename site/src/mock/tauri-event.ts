import { emit as busEmit, on } from "./events";

export type UnlistenFn = () => void;

export async function listen<T = unknown>(name: string, cb: (e: { payload: T }) => void): Promise<UnlistenFn> {
  return on(name, cb as (e: { payload: unknown }) => void);
}

export async function emit(name: string, payload?: unknown): Promise<void> {
  busEmit(name, payload);
}
