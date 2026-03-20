import { useEffect, useRef } from "react";
import { useStore } from "../store";
import { playSound } from "../lib/sounds";
import type { Task, TaskStatus } from "../types/task";

function notify(title: string, body: string): void {
  if (typeof window !== "undefined" && "Notification" in window) {
    if (Notification.permission === "granted") {
      new Notification(title, { body });
    }
  }
}

/**
 * Handle a server-pushed notification event.
 * Maps the notification `kind` to a sound, then triggers browser notification.
 */
export function notifyFromServer(kind: string, title: string, body: string): void {
  notify(title, body);
  const soundMap: Record<string, "permission" | "complete" | "error"> = {
    info: "complete",
    warning: "permission",
    error: "error",
    budget_alert: "permission",
    gate_failure: "error",
    task_complete: "complete",
  };
  const sound = soundMap[kind];
  if (sound) {
    playSound(sound);
  }
}

function updateBadge(inputCount: number): void {
  if (typeof document !== "undefined") {
    document.title = inputCount > 0 ? `(${inputCount}) Shepherd` : "Shepherd";
  }
}

/**
 * Watches the Zustand store for task status transitions and triggers
 * browser notifications, sounds, and document title badge updates.
 *
 * Tauri-native notifications, tray, and dock badge will be wired in Plan 3.
 */
export function useNotifications(): void {
  const prevTasksRef = useRef<Record<number, Task>>({});

  useEffect(() => {
    const unsubscribe = useStore.subscribe((state) => {
      const prevTasks = prevTasksRef.current;
      const currentTasks = state.tasks;

      // Detect status transitions
      for (const [idStr, task] of Object.entries(currentTasks)) {
        const id = Number(idStr);
        const prev = prevTasks[id];

        if (!prev) continue; // New task, no transition to detect
        if (prev.status === task.status) continue; // No change

        const newStatus: TaskStatus = task.status;

        if (newStatus === "input") {
          notify("Permission Required", `"${task.title}" needs your input`);
          playSound("permission");
        } else if (newStatus === "done") {
          notify("Task Complete", `"${task.title}" finished successfully`);
          playSound("complete");
        } else if (newStatus === "error") {
          notify("Task Error", `"${task.title}" encountered an error`);
          playSound("error");
        }
      }

      // Update badge with count of tasks needing input
      const inputCount = Object.values(currentTasks).filter(
        (t) => t.status === "input",
      ).length;
      updateBadge(inputCount);

      // Snapshot current state for next comparison
      prevTasksRef.current = { ...currentTasks };
    });

    return unsubscribe;
  }, []);
}
