import { create } from "zustand";
import { createTasksSlice, type TasksSlice } from "./tasks";
import { createSessionsSlice, type SessionsSlice } from "./sessions";
import { createUiSlice, type UiSlice } from "./ui";

export type ShepherdStore = TasksSlice & SessionsSlice & UiSlice;

export const useStore = create<ShepherdStore>()((...a) => ({
  ...createTasksSlice(...a),
  ...createSessionsSlice(...a),
  ...createUiSlice(...a),
}));

export const useTasks = () => useStore((s) => s.tasks);
export const usePendingPermissions = () => useStore((s) => s.pendingPermissions);
export const useViewMode = () => useStore((s) => s.viewMode);
export const useFocusedTaskId = () => useStore((s) => s.focusedTaskId);
export const useConnectionStatus = () => useStore((s) => s.connectionStatus);
