import { useEffect, useRef } from "react";
import { useStore } from "../store";
import { playSound } from "../lib/sounds";
import { invoke } from "../lib/tauri";
import type { Task, TaskStatus } from "../types/task";

const isTauri = typeof window !== "undefined" && "__TAURI__" in window;

function notify(title: string, body: string): void {
  if (typeof window !== "undefined" && "Notification" in window) {
    if (Notification.permission === "granted") {
      new Notification(title, { body });
    }
  }
  // Tauri native notification (system notification center, sounds)
  if (isTauri) {
    invoke("plugin:notification|notify", { title, body }).catch(() => {});
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

function updateBadge(running: number, input: number, error: number): void {
  if (typeof document !== "undefined") {
    document.title = input > 0 ? `(${input}) Shepherd` : "Shepherd";
  }
  if (isTauri) {
    invoke("set_dock_badge", { text: input > 0 ? String(input) : "" }).catch(() => {});
    invoke("update_tray_status", { running, input, error }).catch(() => {});
  }
}

/**
 * Watches the Zustand store for task status transitions and triggers
 * browser notifications, sounds, and document title badge updates.
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

      // Update badge and tray with task counts
      const tasks = Object.values(currentTasks);
      const inputCount = tasks.filter((t) => t.status === "input").length;
      const runningCount = tasks.filter((t) => t.status === "running").length;
      const errorCount = tasks.filter((t) => t.status === "error").length;
      updateBadge(runningCount, inputCount, errorCount);

      // Snapshot current state for next comparison
      prevTasksRef.current = { ...currentTasks };
    });

    return unsubscribe;
  }, []);
}
