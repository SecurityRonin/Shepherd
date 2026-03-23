import React, { useState, useEffect, useRef } from "react";
import type { TaskStatus } from "../../types/task";

interface TaskSummaryProps {
  taskId: number;
  taskStatus: TaskStatus;
}

export const TaskSummary: React.FC<TaskSummaryProps> = ({ taskId, taskStatus }) => {
  const [summary, setSummary] = useState<string | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const cachedTaskId = useRef<number | null>(null);

  useEffect(() => {
    if (taskStatus !== "done") return;
    if (cachedTaskId.current === taskId && summary) return;

    setLoading(true);
    setError(null);
    import("../../lib/api")
      .then(({ getTaskSummary }) => getTaskSummary(taskId))
      .then((data) => {
        setSummary(data.summary);
        cachedTaskId.current = taskId;
      })
      .catch((err) => setError(err instanceof Error ? err.message : "Failed to load summary"))
      .finally(() => setLoading(false));
  }, [taskId, taskStatus]);

  if (taskStatus !== "done") return null;
  if (loading) return <div data-testid="summary-loading" className="px-4 py-3 border-b border-shepherd-border bg-shepherd-surface/50 text-sm text-shepherd-muted">Generating summary...</div>;
  if (error) return <div data-testid="summary-error" className="px-4 py-3 border-b border-shepherd-border bg-shepherd-surface/50 text-sm text-shepherd-red">Summary unavailable</div>;
  if (!summary) return null;

  return (
    <div data-testid="task-summary" className="px-4 py-3 border-b border-shepherd-border bg-shepherd-surface/50">
      <h3 className="text-xs font-semibold text-shepherd-muted uppercase tracking-wider mb-1">Summary</h3>
      <p className="text-sm text-shepherd-text leading-relaxed">{summary}</p>
    </div>
  );
};
