import React from "react";
import { useStore } from "../../store";
import type { ConnectionStatus } from "../../lib/ws";

const STATUS_COLORS: Record<ConnectionStatus, string> = {
  connected: "bg-shepherd-green",
  connecting: "bg-shepherd-yellow",
  reconnecting: "bg-shepherd-yellow animate-pulse",
  disconnected: "bg-shepherd-red",
};

const STATUS_LABELS: Record<ConnectionStatus, string> = {
  connected: "Connected",
  connecting: "Connecting...",
  reconnecting: "Reconnecting...",
  disconnected: "Disconnected",
};

export const Header: React.FC = () => {
  const viewMode = useStore((s) => s.viewMode);
  const connectionStatus = useStore((s) => s.connectionStatus);
  const setNewTaskDialogOpen = useStore((s) => s.setNewTaskDialogOpen);
  const exitFocus = useStore((s) => s.exitFocus);
  const pendingPermissions = useStore((s) => s.pendingPermissions);
  const needsInputCount = pendingPermissions.length;

  return (
    <header className="h-12 flex items-center justify-between px-4 border-b border-shepherd-border bg-shepherd-surface shrink-0">
      <div className="flex items-center gap-3">
        {viewMode === "focus" && (
          <button onClick={exitFocus} className="text-shepherd-muted hover:text-shepherd-text text-sm flex items-center gap-1 transition-colors">
            <span className="text-xs">&larr;</span> Overview
          </button>
        )}
        <h1 className="text-sm font-semibold text-shepherd-text tracking-wide uppercase">Shepherd</h1>
        <span className="text-xs text-shepherd-muted">{viewMode === "overview" ? "Overview" : "Focus"}</span>
      </div>
      <div className="flex items-center gap-2">
        {needsInputCount > 0 && (
          <span className="px-2 py-0.5 text-xs rounded-full bg-shepherd-orange/20 text-shepherd-orange font-medium">
            {needsInputCount} pending
          </span>
        )}
      </div>
      <div className="flex items-center gap-3">
        <button onClick={() => setNewTaskDialogOpen(true)} className="px-3 py-1.5 text-xs font-medium rounded bg-shepherd-accent text-white hover:bg-shepherd-accent/80 transition-colors" title="New Task (Cmd+N)">
          + New Task
        </button>
        <div className="flex items-center gap-1.5" title={STATUS_LABELS[connectionStatus]}>
          <div className={`w-2 h-2 rounded-full ${STATUS_COLORS[connectionStatus]}`} />
          <span className="text-xs text-shepherd-muted">{STATUS_LABELS[connectionStatus]}</span>
        </div>
      </div>
    </header>
  );
};
