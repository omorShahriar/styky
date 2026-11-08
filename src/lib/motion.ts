import type { Transition } from "motion/react";

/** Springs only (PLAN §5). No linear easing anywhere in the UI. */
const reduced =
  typeof window !== "undefined" && window.matchMedia?.("(prefers-reduced-motion: reduce)").matches;

/** Reduced motion: no movement, just a quick fade (an ease-out, never linear). */
const fade: Transition = { duration: 0.14, ease: [0.23, 1, 0.32, 1] };

export const expandSpring: Transition = reduced ? fade : { type: "spring", stiffness: 300, damping: 30 };
export const pressSpring: Transition = reduced ? fade : { type: "spring", stiffness: 400, damping: 32 };

/** Row height/opacity for enter + exit (completion collapses the row). */
export const rowVariants = reduced
  ? { initial: { opacity: 0 }, animate: { opacity: 1 }, exit: { opacity: 0 } }
  : {
      initial: { opacity: 0, height: 0 },
      animate: { opacity: 1, height: "auto" },
      exit: { opacity: 0, height: 0 },
    };
