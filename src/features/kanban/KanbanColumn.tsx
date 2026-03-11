import React from "react";
import type { Task, TaskStatus } from "../../types/task";

export interface KanbanColumnProps {
  status: TaskStatus;
  label: string;
  tasks: Task[];
  renderCard: (task: Task) => React.ReactNode;
  isDraggable?: boolean;
  accentColor: string;
}

const COLUMN_BG: Record<TaskStatus, string> = {
  queued: "border-shepherd-muted/30",
  running: "border-shepherd-accent/30",
  input: "border-shepherd-orange/30",
  review: "border-shepherd-purple/30",
  done: "border-shepherd-green/30",
  error: "border-shepherd-red/30",
};

export const KanbanColumn: React.FC<KanbanColumnProps> = ({
  status,
  label,
  tasks,
  renderCard,
  accentColor,
}) => {
  return (
    <div className={`flex min-w-[260px] max-w-[320px] flex-1 flex-col rounded-lg bg-shepherd-surface/50 border ${COLUMN_BG[status]}`}>
      {/* Header */}
      <div className="flex items-center gap-2 px-3 py-2.5 border-b border-shepherd-border/50">
        <span
          className="inline-block h-2.5 w-2.5 rounded-full shrink-0"
          style={{ backgroundColor: accentColor }}
        />
        <h2 className="text-xs font-semibold text-shepherd-text uppercase tracking-wider flex-1">{label}</h2>
        <span className="ml-auto inline-flex h-5 min-w-[20px] items-center justify-center rounded-full bg-shepherd-border px-1.5 text-xs font-medium text-shepherd-muted">
          {tasks.length}
        </span>
      </div>

      {/* Card list */}
      <div className="flex-1 overflow-y-auto p-2 space-y-2">
        {tasks.length === 0 ? (
          <div className="flex items-center justify-center py-8 text-xs text-shepherd-muted">
            No tasks
          </div>
        ) : (
          tasks.map((task) => renderCard(task))
        )}
      </div>
    </div>
  );
};
