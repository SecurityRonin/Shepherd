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
        <h1 className="flex items-center gap-2 text-sm font-semibold text-shepherd-text tracking-wide uppercase">
          <svg className="w-5 h-5 fill-current" viewBox="0 0 450 450.4" aria-hidden="true">
            <path d="M141.26,417.46l82.95-240.12,28.82-85.08c3.45-10.19,2.13-20.58-3.84-29.35-5.36-7.88-14.46-13.61-25.11-14.02-11.06-.43-20.7,4.9-26.53,12.78-6.75,9.14-7.63,20.13-3.68,30.6,4.72,12.5-2.73,24.4-13.2,28.47-9.61,3.74-19.13,1.57-26.3-5.08-5.93-5.51-9.26-12.76-11.03-21.01-7.11-33.11,8.36-67.08,38.22-83.99,24.84-14.06,55.04-14.43,81.35.25,21.88,12.21,39.29,36.08,41.53,63.58,1,12.31-.74,24.59-4.8,36.54l-30.91,91.04-44.38,128.74-34.06,98.45c-4.54,13.13-14.42,23.14-29.61,20.81-14.09-2.16-24.68-17.44-19.44-32.61Z"/><path d="M315.68,299.64c-3.43-7.89-2.55-18.68,3.84-25.04l47.57-47.35-44.83-44.24c-8.76-8.65-11.18-21.04-4.72-31.48,9.58-12.56,27.3-13.46,38.49-2.38l56.78,56.21c11.3,11.19,14.46,28.88,2.16,41.2l-59.19,59.23c-6,6-13.48,8.71-22.06,7.48-6.94-.99-14.42-5.29-18.05-13.62Z"/><path d="M141.49,302.45c-8.53,12.89-27.07,15.27-38.16,4.11l-59.04-59.43c-12.34-12.42-9.89-30.45,1.78-41.93l58.37-57.39c6.04-5.94,14.72-7.63,22.8-5.74,7.09,1.65,14.12,7.17,16.73,15.15,3.07,9.37-.44,18.76-7.39,25.56l-45.33,44.29,44.32,43.39c8.76,8.58,12.59,20.36,5.91,31.99Z"/>
          </svg>
          Shepherd
        </h1>
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
