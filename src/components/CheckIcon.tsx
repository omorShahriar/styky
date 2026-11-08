import { motion } from "motion/react";

/** Checkmark that draws itself in. */
export function CheckIcon({ size = 10 }: { size?: number }) {
  return (
    <svg width={size} height={size} viewBox="0 0 12 12" fill="none" aria-hidden>
      <motion.path
        d="M2.5 6.4 5 8.8 9.6 3.4"
        stroke="currentColor"
        strokeWidth={1.8}
        strokeLinecap="round"
        strokeLinejoin="round"
        initial={{ pathLength: 0 }}
        animate={{ pathLength: 1 }}
        transition={{ type: "spring", stiffness: 300, damping: 30 }}
      />
    </svg>
  );
}
