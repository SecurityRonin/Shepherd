import type { StateCreator } from "zustand";
import type { ConnectionStatus } from "../lib/ws";

export type ViewMode = "overview" | "focus";

export interface UiSlice {
  viewMode: ViewMode;
  focusedTaskId: number | null;
  connectionStatus: ConnectionStatus;
  isNewTaskDialogOpen: boolean;
  isCommandPaletteOpen: boolean;
  focusedPanel: "terminal" | "changes";
  setViewMode: (mode: ViewMode) => void;
  setFocusedTaskId: (id: number | null) => void;
  enterFocus: (taskId: number) => void;
  exitFocus: () => void;
  toggleView: () => void;
  setConnectionStatus: (status: ConnectionStatus) => void;
  setNewTaskDialogOpen: (open: boolean) => void;
  setCommandPaletteOpen: (open: boolean) => void;
  setFocusedPanel: (panel: "terminal" | "changes") => void;
}

export const createUiSlice: StateCreator<UiSlice, [], [], UiSlice> = (set, get) => ({
  viewMode: "overview",
  focusedTaskId: null,
  connectionStatus: "disconnected",
  isNewTaskDialogOpen: false,
  isCommandPaletteOpen: false,
  focusedPanel: "terminal",
  setViewMode: (mode) => set({ viewMode: mode }),
  setFocusedTaskId: (id) => set({ focusedTaskId: id }),
  enterFocus: (taskId) =>
    set({ viewMode: "focus", focusedTaskId: taskId, focusedPanel: "terminal" }),
  exitFocus: () => set({ viewMode: "overview", focusedTaskId: null }),
  toggleView: () => {
    const { viewMode, focusedTaskId } = get();
    if (viewMode === "overview" && focusedTaskId !== null) {
      set({ viewMode: "focus" });
    } else {
      set({ viewMode: "overview" });
    }
  },
  setConnectionStatus: (status) => set({ connectionStatus: status }),
  setNewTaskDialogOpen: (open) => set({ isNewTaskDialogOpen: open }),
  setCommandPaletteOpen: (open) => set({ isCommandPaletteOpen: open }),
  setFocusedPanel: (panel) => set({ focusedPanel: panel }),
});
