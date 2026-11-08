/** Tiny pub/sub standing in for Tauri's event system. */
type Handler = (e: { payload: unknown }) => void;
const handlers = new Map<string, Set<Handler>>();

export function on(name: string, cb: Handler): () => void {
  let set = handlers.get(name);
  if (!set) handlers.set(name, (set = new Set()));
  set.add(cb);
  return () => set.delete(cb);
}

export function emit(name: string, payload?: unknown) {
  handlers.get(name)?.forEach((h) => h({ payload }));
}
