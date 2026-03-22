import React, { useCallback, useEffect } from "react";
import { ErrorBoundary } from "./features/shared/ErrorBoundary";
import { Layout } from "./features/shared/Layout";
import { KanbanBoard } from "./features/kanban/KanbanBoard";
import { FocusView } from "./features/focus/FocusView";
import { CostDashboard } from "./features/observability/CostDashboard";
import { ReplayViewer } from "./features/replay/ReplayViewer";
import { EcosystemManager } from "./features/ecosystem/EcosystemManager";
import { CloudSettings } from "./features/cloud/CloudSettings";
import { TemplateGallery } from "./features/templates/TemplateGallery";
import { CommandPalette } from "./features/palette/CommandPalette";
import { NewTaskDialog } from "./features/tasks/NewTaskDialog";
import { useWebSocket } from "./hooks/useWebSocket";
import { useKeyboardShortcuts } from "./hooks/useKeyboardShortcuts";
import { useNotifications, notifyFromServer } from "./hooks/useNotifications";
import { useAuthCallback } from "./hooks/useAuthCallback";
import { useStore } from "./store";
import type { ServerEvent } from "./types";
import type { ConnectionStatus } from "./lib/ws";

const App: React.FC = () => {
  const viewMode = useStore((s) => s.viewMode);
  const isCommandPaletteOpen = useStore((s) => s.isCommandPaletteOpen);
  const setCommandPaletteOpen = useStore((s) => s.setCommandPaletteOpen);
  const isNewTaskDialogOpen = useStore((s) => s.isNewTaskDialogOpen);
  const setNewTaskDialogOpen = useStore((s) => s.setNewTaskDialogOpen);
  const focusedTaskId = useStore((s) => s.focusedTaskId);

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
        store.dispatchTerminalOutput(event.data.task_id, event.data.data);
        break;
      case "gate_result":
        store.handleGateResult(event.data);
        break;
      case "metrics_update":
        store.handleMetricsUpdate(event.data);
        break;
      case "budget_alert":
        store.handleBudgetAlert(event.data);
        break;
      case "notification":
        notifyFromServer(event.data.kind, event.data.title, event.data.body);
        break;
    }
  }, []);

  const handleStatusChange = useCallback((status: ConnectionStatus) => {
    useStore.getState().setConnectionStatus(status);
  }, []);

  const wsRef = useWebSocket(handleServerEvent, handleStatusChange);
  useKeyboardShortcuts(wsRef);
  useNotifications();
  useAuthCallback();

  // Sync the wsClient ref into the store whenever connection status changes.
  // The wsRef.current is set before onStatusChange fires, so it's safe to read here.
  const connectionStatus = useStore((s) => s.connectionStatus);
  useEffect(() => {
    useStore.getState().setWsClient(wsRef.current);
  }, [connectionStatus, wsRef]);

  const renderView = () => {
    switch (viewMode) {
      case "focus": return <FocusView />;
      case "observability": return <CostDashboard />;
      case "replay": return <ReplayViewer taskId={focusedTaskId ?? undefined} />;
      case "ecosystem": return <EcosystemManager />;
      case "cloud": return <CloudSettings />;
      case "templates": return <TemplateGallery />;
      default: return <KanbanBoard />;
    }
  };

  return (
    <ErrorBoundary>
      <Layout>
        {renderView()}
        <CommandPalette
          isOpen={isCommandPaletteOpen}
          onClose={() => setCommandPaletteOpen(false)}
        />
        <NewTaskDialog
          isOpen={isNewTaskDialogOpen}
          onClose={() => setNewTaskDialogOpen(false)}
        />
      </Layout>
    </ErrorBoundary>
  );
};

export default App;
