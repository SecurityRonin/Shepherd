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

export const KanbanColumn: React.FC<KanbanColumnProps> = ({
  label,
  tasks,
  renderCard,
  accentColor,
}) => {
  return (
    <div className="flex min-w-[240px] flex-1 flex-col rounded-lg bg-shepherd-surface/50">
      {/* Header */}
      <div className="flex items-center gap-2 px-3 py-2 border-b border-shepherd-border">
        <span
          className="inline-block h-2.5 w-2.5 rounded-full"
          style={{ backgroundColor: accentColor }}
        />
        <span className="text-sm font-medium text-shepherd-text">{label}</span>
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
