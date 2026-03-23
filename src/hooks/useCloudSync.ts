import { useEffect, useRef, useCallback } from "react";
import { useStore } from "../store";
import { getCloudStatus, syncTasksToCloud } from "../lib/api";

export function useCloudSync() {
  const syncTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const isSyncing = useRef(false);
  const isAuthenticated = useRef(false);

  const pullFromCloud = useCallback(async () => {
    try {
      const status = await getCloudStatus();
      if (!status.cloud_available || !status.authenticated) {
        isAuthenticated.current = false;
        return;
      }
      isAuthenticated.current = true;
      // Pull is handled through the existing sync mechanism.
      // The Rust backend's cloud module already syncs on auth.
    } catch {
      isAuthenticated.current = false;
    }
  }, []);

  const pushToCloud = useCallback(async () => {
    if (!isAuthenticated.current || isSyncing.current) return;
    isSyncing.current = true;
    try {
      const tasks = useStore.getState().tasks;
      const taskList = Object.values(tasks);
      if (taskList.length === 0) return;
      await syncTasksToCloud(taskList);
    } catch (err) {
      console.error("[cloud-sync] Push failed:", err);
    } finally {
      isSyncing.current = false;
    }
  }, []);

  useEffect(() => {
    pullFromCloud();

    const unsubscribe = useStore.subscribe((state, prevState) => {
      if (!isAuthenticated.current) return;
      if (state.tasks === prevState.tasks) return;

      if (syncTimerRef.current) clearTimeout(syncTimerRef.current);
      syncTimerRef.current = setTimeout(() => {
        pushToCloud();
      }, 5000);
    });

    return () => {
      unsubscribe();
      if (syncTimerRef.current) {
        clearTimeout(syncTimerRef.current);
        syncTimerRef.current = null;
      }
    };
  }, [pullFromCloud, pushToCloud]);
}
