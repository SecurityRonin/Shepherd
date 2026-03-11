import React, { useCallback } from "react";
import { Layout } from "./features/shared/Layout";
import { useWebSocket } from "./hooks/useWebSocket";
import { useKeyboardShortcuts } from "./hooks/useKeyboardShortcuts";
import { useStore } from "./store";
import type { ServerEvent } from "./types";
import type { ConnectionStatus } from "./lib/ws";

const App: React.FC = () => {
  const viewMode = useStore((s) => s.viewMode);

  const handleServerEvent = useCallback((event: ServerEvent) => {
    const store = useStore.getState();
    switch (event.type) {
      case "status_snapshot":
        store.setTasks(event.data.tasks);
        store.setPendingPermissions(event.data.pending_permissions);
        break;
      case "task_created":
      case "task_updated":
        store.upsertTask(event.data);
        break;
      case "task_deleted":
        store.removeTask(event.data.id);
        break;
      case "permission_requested":
        store.addPendingPermission(event.data);
        break;
      case "permission_resolved":
        store.removePendingPermission(event.data.id);
        break;
      case "terminal_output":
      case "gate_result":
      case "notification":
        break;
    }
  }, []);

  const handleStatusChange = useCallback((status: ConnectionStatus) => {
    useStore.getState().setConnectionStatus(status);
  }, []);

  const wsRef = useWebSocket(handleServerEvent, handleStatusChange);
  useKeyboardShortcuts(wsRef);

  return (
    <Layout>
      {viewMode === "overview" ? (
        <div className="flex items-center justify-center h-full text-shepherd-muted">
          <p className="text-sm">Kanban board will render here (Task 8)</p>
        </div>
      ) : (
        <div className="flex items-center justify-center h-full text-shepherd-muted">
          <p className="text-sm">Focus panel will render here (Task 11)</p>
        </div>
      )}
    </Layout>
  );
};

export default App;
