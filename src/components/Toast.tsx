import { AnimatePresence, motion } from "motion/react";
import { useStore } from "../lib/store";
import { expandSpring } from "../lib/motion";

export function Toast() {
  const toast = useStore((s) => s.toast);
  return (
    <div className="toast-host" aria-live="polite">
      <AnimatePresence>
        {toast && (
          <motion.div
            key={toast.id}
            className="toast"
            role="status"
            initial={{ opacity: 0, y: 12, scale: 0.96 }}
            animate={{ opacity: 1, y: 0, scale: 1 }}
            exit={{ opacity: 0, y: 8, scale: 0.97 }}
            transition={expandSpring}
          >
            <span className="toast-msg">{toast.message}</span>
            {toast.action && (
              <button type="button" className="toast-action" onClick={toast.action.run}>
                {toast.action.label}
              </button>
            )}
          </motion.div>
        )}
      </AnimatePresence>
    </div>
  );
}
