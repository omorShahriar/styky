/** Back-off for the first loads at startup, when the backend may not be ready to answer yet. */
export const RETRY_STEPS_MS = [250, 500, 1000, 2000, 3000];

/** Delay before retry number `attempt` (1-based); stays at the last step. */
export function retryDelay(attempt: number): number {
  return RETRY_STEPS_MS[Math.min(Math.max(attempt, 1), RETRY_STEPS_MS.length) - 1];
}

/** Failures that stay silent before the user is told. */
export const QUIET_ATTEMPTS = 4;
