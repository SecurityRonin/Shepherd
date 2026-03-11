import { useState, useEffect } from "react";

export type StalenessLevel = "fresh" | "stale" | "critical";

/** Tasks updated within this window are "fresh" */
export const FRESH_THRESHOLD_MS = 30_000; // 30 seconds

/** Tasks updated within this window (but past FRESH) are "stale" */
export const STALE_THRESHOLD_MS = 120_000; // 2 minutes

/** Re-evaluation interval */
const TICK_INTERVAL_MS = 10_000; // 10 seconds

function computeLevel(updatedAt: string, isActive: boolean): StalenessLevel {
  if (!isActive) return "fresh";

  const elapsed = Date.now() - new Date(updatedAt).getTime();
  if (elapsed > STALE_THRESHOLD_MS) return "critical";
  if (elapsed > FRESH_THRESHOLD_MS) return "stale";
  return "fresh";
}

/**
 * Hook that tracks how stale a task is based on its updated_at timestamp.
 * Only evaluates staleness for active tasks (running/input).
 * Re-evaluates every 10 seconds.
 */
export function useTaskStaleness(
  updatedAt: string,
  isActive: boolean,
): StalenessLevel {
  const [level, setLevel] = useState<StalenessLevel>(() =>
    computeLevel(updatedAt, isActive),
  );

  useEffect(() => {
    setLevel(computeLevel(updatedAt, isActive));

    if (!isActive) return;

    const interval = setInterval(() => {
      setLevel(computeLevel(updatedAt, isActive));
    }, TICK_INTERVAL_MS);

    return () => clearInterval(interval);
  }, [updatedAt, isActive]);

  return level;
}
