import { useEffect, useRef, useState } from "react";
import { AnimatePresence, Reorder } from "motion/react";
import { api } from "../lib/api";
import { useStore } from "../lib/store";
import type { Item } from "../lib/types";
import { ItemRow } from "./ItemRow";

/** The trailing group of manually ordered (no due date, not urgent) items: drag to reorder. */
export function PlainGroup({ items }: { items: Item[] }) {
  const [order, setOrder] = useState(() => items.map((i) => i.id));
  const orderRef = useRef(order);
  orderRef.current = order;

  // Follow the backend whenever the set or order of items changes.
  const key = items.map((i) => i.id).join("|");
  useEffect(() => {
    setOrder(items.map((i) => i.id));
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [key]);

  const byId = new Map(items.map((i) => [i.id, i]));
  const commit = () => {
    const ids = orderRef.current;
    if (ids.join("|") === key) return;
    api
      .reorder(ids)
      .then(() => useStore.getState().refresh())
      .catch((e) => useStore.getState().showToast(String(e)));
  };

  return (
    <Reorder.Group as="ul" axis="y" values={order} onReorder={setOrder} className="list">
      <AnimatePresence initial={false}>
        {order.map((id) => {
          const item = byId.get(id);
          return item ? <ItemRow key={id} item={item} reorderable onDragEnd={commit} /> : null;
        })}
      </AnimatePresence>
    </Reorder.Group>
  );
}
