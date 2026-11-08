import type { Transition } from "motion/react";

/** Springs only (PLAN §5). No linear easing anywhere in the UI. */
export const expandSpring: Transition = { type: "spring", stiffness: 300, damping: 30 };
export const pressSpring: Transition = { type: "spring", stiffness: 400, damping: 32 };

/** Row height/opacity for enter + exit (completion collapses the row). */
export const rowVariants = {
  initial: { opacity: 0, height: 0 },
  animate: { opacity: 1, height: "auto" },
  exit: { opacity: 0, height: 0 },
};
