import { AnimatePresence, motion } from "motion/react";
import { ChevronRight } from "lucide-react";
import { useStore } from "../lib/store";
import { expandSpring } from "../lib/motion";
import type { Item } from "../lib/types";
import { ItemRow } from "./ItemRow";

export function CompletedSection({ items }: { items: Item[] }) {
  const open = useStore((s) => s.completedOpen);
  const toggle = useStore((s) => s.toggleCompleted);
  const days = Number(useStore((s) => s.settings.autoclear_days) ?? 7);
  if (items.length === 0) return null;

  return (
    <section className="completed">
      <button type="button" className="completed-head" aria-expanded={open} onClick={toggle}>
        <motion.span animate={{ rotate: open ? 90 : 0 }} transition={expandSpring} className="chev">
          <ChevronRight size={14} strokeWidth={2.2} />
        </motion.span>
        <span>Completed</span>
        <span className="count tnum">{items.length}</span>
        <span className="completed-hint">{days > 0 ? `cleared after ${days} d` : "kept"}</span>
      </button>
      <AnimatePresence initial={false}>
        {open && (
          <motion.ul
            key="list"
            className="list"
            initial={{ height: 0, opacity: 0 }}
            animate={{ height: "auto", opacity: 1 }}
            exit={{ height: 0, opacity: 0 }}
            transition={expandSpring}
            style={{ overflow: "clip" }}
          >
            {items.map((i) => (
              <ItemRow key={i.id} item={i} />
            ))}
          </motion.ul>
        )}
      </AnimatePresence>
    </section>
  );
}
