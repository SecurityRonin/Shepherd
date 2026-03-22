import React, { useCallback, useMemo, useState } from "react";
import { useStore } from "../../store";
import { KanbanColumn } from "./KanbanColumn";
import { TaskCard } from "./TaskCard";
import type { Task, TaskStatus } from "../../types/task";

interface ColumnDef {
  status: TaskStatus;
  label: string;
  accentColor: string;
}

const COLUMNS: ColumnDef[] = [
  { status: "queued", label: "Queued", accentColor: "#8b949e" },
  { status: "running", label: "Running", accentColor: "#58a6ff" },
  { status: "input", label: "Needs Input", accentColor: "#db6d28" },
  { status: "review", label: "Review", accentColor: "#bc8cff" },
  { status: "done", label: "Done", accentColor: "#3fb950" },
];

const TWENTY_FOUR_HOURS_MS = 24 * 60 * 60 * 1000;

function groupAndSortTasks(
  tasks: Record<number, Task>,
): Record<TaskStatus, Task[]> {
  const grouped: Record<TaskStatus, Task[]> = {
    queued: [],
    running: [],
    input: [],
    review: [],
    error: [],
    done: [],
    cancelled: [],
  };

  for (const task of Object.values(tasks)) {
    grouped[task.status].push(task);
  }

  // Queued: sort by ID ascending
  grouped.queued.sort((a, b) => a.id - b.id);

  // Running, input, review: sort by updated_at descending
  const byUpdatedDesc = (a: Task, b: Task) =>
    new Date(b.updated_at).getTime() - new Date(a.updated_at).getTime();
  grouped.running.sort(byUpdatedDesc);
  grouped.input.sort(byUpdatedDesc);
  grouped.review.sort(byUpdatedDesc);

  // Error tasks fold into review
  grouped.review = [...grouped.review, ...grouped.error.sort(byUpdatedDesc)];
  grouped.error = [];

  // Done: sort by updated_at descending, fade after 24h
  grouped.done.sort(byUpdatedDesc);

  return grouped;
}

export const KanbanBoard: React.FC = () => {
  const tasks = useStore((s) => s.tasks);
  const pendingPermissions = useStore((s) => s.pendingPermissions);
  const enterFocus = useStore((s) => s.enterFocus);

  const [queuedOrder, setQueuedOrder] = useState<number[] | null>(null);

  // Pre-compute permission lookup to avoid O(n*m) filtering per card
  const permissionsByTask = useMemo(() => {
    const map = new Map<number, typeof pendingPermissions>();
    for (const p of pendingPermissions) {
      const existing = map.get(p.task_id);
      if (existing) {
        existing.push(p);
      } else {
        map.set(p.task_id, [p]);
      }
    }
    return map;
  }, [pendingPermissions]);

  const grouped = useMemo(() => {
    const result = groupAndSortTasks(tasks);

    // Apply custom queued order if set
    if (queuedOrder) {
      const queuedById = new Map(result.queued.map((t) => [t.id, t]));
      const ordered: Task[] = [];
      for (const id of queuedOrder) {
        const task = queuedById.get(id);
        if (task) {
          ordered.push(task);
          queuedById.delete(id);
        }
      }
      // Append any new tasks not in the custom order
      for (const task of queuedById.values()) {
        ordered.push(task);
      }
      result.queued = ordered;
    }

    return result;
  }, [tasks, queuedOrder]);

  const handleQueuedReorder = useCallback((newOrder: number[]) => {
    setQueuedOrder(newOrder);
  }, []);

  const renderCard = useCallback(
    (task: Task) => {
      const taskPerms = permissionsByTask.get(task.id) ?? [];
      const isDoneFaded =
        task.status === "done" &&
        Date.now() - new Date(task.updated_at).getTime() >
          TWENTY_FOUR_HOURS_MS;

      return (
        <div
          key={task.id}
          className={isDoneFaded ? "opacity-40" : undefined}
        >
          <TaskCard
            task={task}
            pendingPermissions={taskPerms}
            onClick={() => enterFocus(task.id)}
          />
        </div>
      );
    },
    [permissionsByTask, enterFocus],
  );

  return (
    <div className="flex h-full gap-3 overflow-x-auto p-4">
      {COLUMNS.map((col) => {
        const columnTasks = grouped[col.status] ?? [];
        return (
          <KanbanColumn
            key={col.status}
            status={col.status}
            label={col.label}
            tasks={columnTasks}
            accentColor={col.accentColor}
            isDraggable={col.status === "queued"}
            onReorder={col.status === "queued" ? handleQueuedReorder : undefined}
            renderCard={renderCard}
          />
        );
      })}
    </div>
  );
};
