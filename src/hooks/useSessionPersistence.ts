import { useState, useEffect, useCallback } from "react";
import type { SessionState } from "../types/task";
import { getInterruptedSessions, clearSessionState } from "../lib/api";

export function useSessionPersistence() {
  const [sessions, setSessions] = useState<SessionState[]>([]);

  useEffect(() => {
    getInterruptedSessions()
      .then((data) => setSessions(data))
      .catch(() => setSessions([]));
  }, []);

  const removeSession = useCallback((taskId: number) => {
    setSessions((prev) => prev.filter((s) => s.task_id !== taskId));
  }, []);

  const resume = useCallback(
    async (taskId: number) => {
      removeSession(taskId);
    },
    [removeSession],
  );

  const startFresh = useCallback(
    async (taskId: number) => {
      await clearSessionState(taskId);
      removeSession(taskId);
    },
    [removeSession],
  );

  const dismiss = useCallback(
    async (taskId: number) => {
      await clearSessionState(taskId);
      removeSession(taskId);
    },
    [removeSession],
  );

  return { sessions, resume, startFresh, dismiss };
}
