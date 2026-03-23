import React, { useState, useCallback, useRef, useEffect, Suspense } from "react";
import { useStore } from "../../store";
import { AgentBadge } from "../shared/AgentBadge";
import { LazyFallback } from "../shared/LazyFallback";
import { SessionSidebar } from "./SessionSidebar";
import { PermissionPrompt } from "./PermissionPrompt";
import { SessionPicker } from "../iterm2/SessionPicker";
import { SetupPrompt } from "../iterm2/SetupPrompt";
import { getClaudeSessions, resumeClaudeSession, startFreshSession } from "../../lib/api";
import { TaskSummary } from "./TaskSummary";

const Terminal = React.lazy(() =>
  import("./Terminal").then((m) => ({ default: m.Terminal })),
);
const DiffViewer = React.lazy(() =>
  import("./DiffViewer").then((m) => ({ default: m.DiffViewer })),
);

const STATUS_COLORS: Record<string, string> = {
  queued: "bg-shepherd-muted",
  running: "bg-shepherd-accent animate-pulse",
  input: "bg-shepherd-orange animate-pulse",
  review: "bg-shepherd-purple",
  error: "bg-shepherd-red",
  done: "bg-shepherd-green",
  cancelled: "bg-shepherd-muted",
};

export function formatTimeSince(dateStr: string): string {
  const now = Date.now();
  const then = new Date(dateStr).getTime();
  const diffMs = now - then;

  if (diffMs < 0) return "just now";

  const seconds = Math.floor(diffMs / 1000);
  if (seconds < 60) return `${seconds}s ago`;

  const minutes = Math.floor(seconds / 60);
  if (minutes < 60) return `${minutes}m ago`;

  const hours = Math.floor(minutes / 60);
  return `${hours}h ago`;
}

export const FocusView: React.FC = () => {
  const focusedTaskId = useStore((s) => s.focusedTaskId);
  const tasks = useStore((s) => s.tasks);

  const task = focusedTaskId !== null ? tasks[focusedTaskId] : undefined;

  // iTerm2 session picker state
  const [claudeSessions, setClaudeSessions] = useState<string[]>([]);
  const [setupDismissed, setSetupDismissed] = useState(false);
  const [sessionError, setSessionError] = useState<string | null>(null);

  useEffect(() => {
    if (!task?.iterm2_session_id) return;
    getClaudeSessions(task.id)
      .then(data => { setSessionError(null); setClaudeSessions(data.sessions ?? []); })
      .catch((err) => setSessionError(err instanceof Error ? err.message : "Failed to load sessions"));
  }, [task?.id, task?.iterm2_session_id]);

  // Resizable right panel state
  const [rightPanelWidth, setRightPanelWidth] = useState(400);
  const isDragging = useRef(false);
  const startX = useRef(0);
  const startWidth = useRef(0);

  const handleMouseDown = useCallback(
    (e: React.MouseEvent) => {
      isDragging.current = true;
      startX.current = e.clientX;
      startWidth.current = rightPanelWidth;

      const handleMouseMove = (e: MouseEvent) => {
        if (!isDragging.current) return;
        const delta = startX.current - e.clientX;
        const newWidth = Math.min(800, Math.max(250, startWidth.current + delta));
        setRightPanelWidth(newWidth);
      };

      const handleMouseUp = () => {
        isDragging.current = false;
        document.removeEventListener("mousemove", handleMouseMove);
        document.removeEventListener("mouseup", handleMouseUp);
      };

      document.addEventListener("mousemove", handleMouseMove);
      document.addEventListener("mouseup", handleMouseUp);
    },
    [rightPanelWidth],
  );

  // No task selected state
  if (!task) {
    return (
      <div className="flex h-full">
        <SessionSidebar />
        <div className="flex-1 flex items-center justify-center text-shepherd-muted">
          <p className="text-sm">No task selected</p>
        </div>
      </div>
    );
  }

  const hasPermissionPrompt = task.status === "input";
  const dotColor = STATUS_COLORS[task.status] ?? "bg-shepherd-muted";

  return (
    <div className="flex h-full overflow-hidden">
      {/* Left panel: SessionSidebar */}
      <SessionSidebar />

      {/* Center + Right panels */}
      <div className="flex-1 flex flex-col min-w-0">
        {/* Task header bar */}
        <div className="flex items-center gap-3 px-4 py-2 bg-shepherd-surface border-b border-shepherd-border">
          <span className={`w-2 h-2 rounded-full flex-shrink-0 ${dotColor}`} />
          <h2 className="text-sm font-medium text-shepherd-text truncate">
            {task.title}
          </h2>
          <AgentBadge agentId={task.agent_id} />
          <span className="text-[11px] text-shepherd-muted font-mono">
            {task.branch}
          </span>
          <span className="text-[11px] text-shepherd-muted">
            {task.isolation_mode}
          </span>
          <span className="text-[11px] text-shepherd-muted ml-auto">
            {formatTimeSince(task.updated_at)}
          </span>
          <span className="text-[10px] uppercase font-semibold px-1.5 py-0.5 rounded bg-shepherd-border text-shepherd-muted">
            {task.status}
          </span>
          {(task.status === "running" || task.status === "input") && (
            <button
              className="ml-2 rounded bg-red-700 px-2 py-0.5 text-[10px] font-medium text-white transition-colors hover:bg-red-600"
              onClick={async () => {
                const { cancelTask } = await import("../../lib/api");
                await cancelTask(task.id).catch(console.error);
              }}
              data-testid="focus-cancel-btn"
            >
              Cancel
            </button>
          )}
        </div>

        {/* AI summary (only rendered for done tasks) */}
        <TaskSummary taskId={task.id} taskStatus={task.status} />

        {/* Content area: Terminal + DiffViewer side by side */}
        <div className="flex-1 flex min-h-0">
          {/* Center panel: Terminal + Permission prompt */}
          <div className="flex-1 flex flex-col min-w-0">
            {/* iTerm2 session controls */}
            {task.iterm2_session_id && (
              <div className="px-4 py-2 border-b border-shepherd-border space-y-2">
                {!setupDismissed && claudeSessions.length === 0 && (
                  <SetupPrompt onDismiss={() => setSetupDismissed(true)} />
                )}
                {sessionError && (
                  <p className="text-xs text-shepherd-red">{sessionError}</p>
                )}
                <SessionPicker
                  taskId={task.id}
                  sessions={claudeSessions}
                  onResume={sessionId => resumeClaudeSession(task.id, sessionId)}
                  onFresh={() => startFreshSession(task.id)}
                />
              </div>
            )}

            {/* Terminal (lazy-loaded — xterm ~500KB) */}
            <Suspense fallback={<LazyFallback label="Loading terminal..." testId="terminal-loading" />}>
              <Terminal taskId={task.id} />
            </Suspense>

            {/* Permission prompt area (shown when task.status === "input") */}
            {hasPermissionPrompt && (
              <PermissionPrompt taskId={task.id} />
            )}
          </div>

          {/* Resize handle */}
          <div
            onMouseDown={handleMouseDown}
            className="w-1 cursor-col-resize bg-shepherd-border hover:bg-shepherd-accent transition-colors flex-shrink-0"
          />

          {/* Right panel: DiffViewer */}
          <div
            style={{ width: rightPanelWidth }}
            className="flex-shrink-0 flex flex-col border-l border-shepherd-border min-h-0"
          >
            <Suspense fallback={<LazyFallback label="Loading diff viewer..." testId="diff-loading" />}>
              <DiffViewer taskId={task.id} />
            </Suspense>
          </div>
        </div>
      </div>
    </div>
  );
};
