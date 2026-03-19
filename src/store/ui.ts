import type { StateCreator } from "zustand";
import type { ConnectionStatus, WsClient } from "../lib/ws";

export type ViewMode = "overview" | "focus" | "observability" | "replay" | "ecosystem" | "cloud";

/** Callback that writes PTY output data to a terminal instance. */
export type TerminalOutputHandler = (data: string) => void;

export interface UiSlice {
  viewMode: ViewMode;
  focusedTaskId: number | null;
  connectionStatus: ConnectionStatus;
  isNewTaskDialogOpen: boolean;
  isCommandPaletteOpen: boolean;
  focusedPanel: "terminal" | "changes";
  /** The active WebSocket client (set once the hook creates it). */
  wsClient: WsClient | null;
  /** Per-task terminal output handlers, keyed by taskId. */
  terminalOutputHandlers: Map<number, TerminalOutputHandler>;
  setViewMode: (mode: ViewMode) => void;
  setFocusedTaskId: (id: number | null) => void;
  enterFocus: (taskId: number) => void;
  exitFocus: () => void;
  toggleView: () => void;
  setConnectionStatus: (status: ConnectionStatus) => void;
  setNewTaskDialogOpen: (open: boolean) => void;
  setCommandPaletteOpen: (open: boolean) => void;
  setFocusedPanel: (panel: "terminal" | "changes") => void;
  setWsClient: (client: WsClient | null) => void;
  registerTerminalHandler: (taskId: number, handler: TerminalOutputHandler) => void;
  unregisterTerminalHandler: (taskId: number) => void;
  /** Dispatch terminal output to the registered handler for a given task. */
  dispatchTerminalOutput: (taskId: number, data: string) => void;
}

export const createUiSlice: StateCreator<UiSlice, [], [], UiSlice> = (set, get) => ({
  viewMode: "overview",
  focusedTaskId: null,
  connectionStatus: "disconnected",
  isNewTaskDialogOpen: false,
  isCommandPaletteOpen: false,
  focusedPanel: "terminal",
  wsClient: null,
  terminalOutputHandlers: new Map(),
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
  setWsClient: (client) => set({ wsClient: client }),
  registerTerminalHandler: (taskId, handler) => {
    const handlers = new Map(get().terminalOutputHandlers);
    handlers.set(taskId, handler);
    set({ terminalOutputHandlers: handlers });
  },
  unregisterTerminalHandler: (taskId) => {
    const handlers = new Map(get().terminalOutputHandlers);
    handlers.delete(taskId);
    set({ terminalOutputHandlers: handlers });
  },
  dispatchTerminalOutput: (taskId, data) => {
    const handler = get().terminalOutputHandlers.get(taskId);
    if (handler) {
      handler(data);
    }
  },
});
