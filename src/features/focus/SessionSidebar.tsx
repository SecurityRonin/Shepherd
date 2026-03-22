import React, { useMemo } from "react";
import { useStore } from "../../store";

const STATUS_COLORS: Record<string, string> = {
  queued: "bg-shepherd-muted",
  running: "bg-shepherd-accent animate-pulse",
  input: "bg-shepherd-orange animate-pulse",
  review: "bg-shepherd-purple",
  error: "bg-shepherd-red",
  done: "bg-shepherd-green",
  cancelled: "bg-shepherd-muted",
};

export const SessionSidebar: React.FC = () => {
  const tasks = useStore((s) => s.tasks);
  const focusedTaskId = useStore((s) => s.focusedTaskId);
  const enterFocus = useStore((s) => s.enterFocus);
  const exitFocus = useStore((s) => s.exitFocus);

  const taskList = useMemo(() => Object.values(tasks), [tasks]);

  return (
    <div className="w-[180px] min-w-[180px] bg-shepherd-bg border-r border-shepherd-border flex flex-col h-full">
      {/* Back button */}
      <button
        onClick={exitFocus}
        className="flex items-center gap-1.5 px-3 py-2 text-shepherd-muted hover:text-shepherd-text text-sm transition-colors border-b border-shepherd-border"
      >
        <span>&larr;</span> Overview
      </button>

      {/* Sessions heading */}
      <div className="px-3 py-2 text-[11px] font-semibold uppercase tracking-wider text-shepherd-muted">
        Sessions
      </div>

      {/* Task list */}
      <div className="flex-1 overflow-y-auto">
        {taskList.map((task) => {
          const isActive = task.id === focusedTaskId;
          const dotColor = STATUS_COLORS[task.status] ?? "bg-shepherd-muted";

          return (
            <button
              key={task.id}
              onClick={() => enterFocus(task.id)}
              className={`w-full text-left px-3 py-1.5 flex items-center gap-2 text-xs transition-colors hover:bg-shepherd-surface/50 ${
                isActive
                  ? "bg-shepherd-surface border-l-2 border-shepherd-accent text-shepherd-text"
                  : "border-l-2 border-transparent text-shepherd-muted"
              }`}
            >
              <span className={`w-2 h-2 rounded-full flex-shrink-0 ${dotColor}`} />
              <span className="truncate">{task.title}</span>
            </button>
          );
        })}
      </div>

      {/* Session count */}
      <div className="px-3 py-2 text-[11px] text-shepherd-muted border-t border-shepherd-border">
        {taskList.length} {taskList.length === 1 ? "session" : "sessions"}
      </div>
    </div>
  );
};
