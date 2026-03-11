import { useEffect, useRef } from "react";
import { createShortcutManager, type ShortcutManager } from "../lib/keys";
import { useStore } from "../store";
import type { WsClient } from "../lib/ws";

export function useKeyboardShortcuts(wsClient: React.MutableRefObject<WsClient | null>): ShortcutManager {
  const managerRef = useRef<ShortcutManager>(createShortcutManager());
  const manager = managerRef.current;

  useEffect(() => {
    const store = useStore.getState;
    manager.register({ id: "toggle-view", label: "Toggle Overview / Focus", keys: "meta+0", handler: () => store().toggleView() });
    manager.register({ id: "new-task", label: "New Task", keys: "meta+n", handler: () => store().setNewTaskDialogOpen(true) });
    manager.register({ id: "approve-current", label: "Approve Current Task", keys: "meta+enter", handler: () => {
      const { focusedTaskId } = store();
      if (focusedTaskId !== null) { wsClient.current?.send({ type: "task_approve", data: { task_id: focusedTaskId } }); }
    }});
    manager.register({ id: "approve-all", label: "Approve All Pending", keys: "meta+shift+enter", handler: () => {
      wsClient.current?.send({ type: "task_approve_all", data: null });
    }});
    manager.register({ id: "focus-terminal", label: "Focus Terminal", keys: "meta+1", viewMode: "focus", handler: () => store().setFocusedPanel("terminal") });
    manager.register({ id: "focus-changes", label: "Focus Changes", keys: "meta+2", viewMode: "focus", handler: () => store().setFocusedPanel("changes") });
    manager.register({ id: "command-palette", label: "Command Palette", keys: "meta+k", handler: () => {
      const current = store().isCommandPaletteOpen;
      store().setCommandPaletteOpen(!current);
    }});
    for (let n = 1; n <= 9; n++) {
      manager.register({ id: `quick-approve-${n}`, label: `Quick Approve Card ${n}`, keys: String(n), viewMode: "overview", handler: () => {
        const inputTasks = store().getTasksByStatus("input");
        const task = inputTasks[n - 1];
        if (task) { wsClient.current?.send({ type: "task_approve", data: { task_id: task.id } }); }
      }});
    }
    const handleKeyDown = (event: KeyboardEvent) => {
      const target = event.target as HTMLElement;
      if ((target.tagName === "INPUT" || target.tagName === "TEXTAREA" || target.isContentEditable) && !event.metaKey && !event.ctrlKey) return;
      const { viewMode } = useStore.getState();
      manager.handleKeyDown(event, viewMode);
    };
    window.addEventListener("keydown", handleKeyDown);
    return () => { window.removeEventListener("keydown", handleKeyDown); };
  }, [manager, wsClient]);

  return manager;
}
