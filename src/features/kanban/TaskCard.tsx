import React, { useCallback, useState } from "react";
import type { Task } from "../../types/task";
import type { PermissionEvent } from "../../types/events";
import { AgentBadge } from "../shared/AgentBadge";
import { StatusIndicator } from "../shared/StatusIndicator";
import { useTaskStaleness } from "../../hooks/useTaskStaleness";
import { Iterm2Badge } from "../iterm2/Iterm2Badge";

export interface TaskCardProps {
  task: Task;
  pendingPermissions?: PermissionEvent[];
  onClick?: () => void;
}

export const TaskCard: React.FC<TaskCardProps> = React.memo(({
  task,
  pendingPermissions = [],
  onClick,
}) => {
  const [approving, setApproving] = useState(false);
  const [cancelling, setCancelling] = useState(false);
  const isActive = task.status === "running" || task.status === "input";
  const stalenessLevel = useTaskStaleness(task.updated_at, isActive);

  const handleApprove = useCallback(
    async (e: React.MouseEvent) => {
      e.stopPropagation();
      if (approving) return;
      setApproving(true);
      try {
        const { approveTask } = await import("../../lib/api");
        await approveTask(task.id);
      } catch (err) {
        console.error("Failed to approve task:", err);
      } finally {
        setApproving(false);
      }
    },
    [task.id, approving],
  );

  const handleCancel = useCallback(
    async (e: React.MouseEvent) => {
      e.stopPropagation();
      if (cancelling) return;
      setCancelling(true);
      try {
        const { cancelTask } = await import("../../lib/api");
        await cancelTask(task.id);
      } catch (err) {
        console.error("Failed to cancel task:", err);
      } finally {
        setCancelling(false);
      }
    },
    [task.id, cancelling],
  );

  const isError = task.status === "error";
  const isInput = task.status === "input";
  const isReview = task.status === "review";

  return (
    <div
      className={`rounded-md border p-3 cursor-pointer transition-colors hover:border-shepherd-text/30 ${
        isError
          ? "border-shepherd-red/40 bg-red-500/5"
          : isInput
            ? "border-shepherd-orange/40 bg-shepherd-surface"
            : "border-shepherd-border bg-shepherd-surface"
      }`}
      onClick={onClick}
      onKeyDown={(e) => {
        if (e.key === "Enter" || e.key === " ") {
          e.preventDefault();
          onClick?.();
        }
      }}
      role="button"
      tabIndex={0}
    >
      {/* Row 1: Title + staleness dot */}
      <div className="flex items-center gap-2">
        <span className="flex-1 text-sm font-medium text-shepherd-text leading-tight line-clamp-2">
          {task.title}
        </span>
        {isActive && (
          <StatusIndicator
            level={stalenessLevel}
            data-testid="staleness-indicator"
          />
        )}
      </div>

      {/* Row 2: Agent badge + branch + iTerm2 badge */}
      <div className="mt-1.5 flex items-center gap-2">
        <AgentBadge agentId={task.agent_id} />
        <span className="truncate text-xs font-mono text-shepherd-muted">
          {task.branch}
        </span>
        {task.iterm2_session_id && <Iterm2Badge />}
      </div>

      {/* Row 3: Status-specific content */}
      {isInput && pendingPermissions.length > 0 && (
        <div className="mt-2 rounded bg-orange-500/10 px-2 py-1 text-xs text-orange-400">
          Awaiting:{" "}
          {pendingPermissions.map((p) => p.tool_name).join(", ")}
        </div>
      )}

      {isReview && (
        <div className="mt-2 text-xs text-purple-400">
          Ready for review
          {task.gate_results && task.gate_results.length > 0 && (
            <div className="mt-1 space-y-0.5">
              {task.gate_results.map((gr) => (
                <div key={gr.gate} className="flex items-center gap-1">
                  <span>{gr.passed ? "pass" : "fail"}</span>
                  <span className="text-shepherd-muted">{gr.gate}</span>
                </div>
              ))}
            </div>
          )}
        </div>
      )}

      {isError && (
        <div className="mt-2 text-xs text-red-400">
          Error — click to investigate
        </div>
      )}

      {/* Row 4: Action buttons */}
      {isInput && (
        <div className="mt-2 flex gap-2">
          <button
            className="flex-1 rounded bg-orange-600 px-2 py-1 text-xs font-medium text-white transition-colors hover:bg-orange-500 disabled:opacity-50"
            onClick={handleApprove}
            disabled={approving}
          >
            {approving ? "Approving..." : "Approve"}
          </button>
          <button
            className="flex-1 rounded bg-red-700 px-2 py-1 text-xs font-medium text-white transition-colors hover:bg-red-600 disabled:opacity-50"
            onClick={handleCancel}
            disabled={cancelling}
            data-testid="cancel-task-btn"
          >
            {cancelling ? "Cancelling..." : "Cancel"}
          </button>
        </div>
      )}
      {task.status === "running" && (
        <button
          className="mt-2 w-full rounded bg-red-700 px-2 py-1 text-xs font-medium text-white transition-colors hover:bg-red-600 disabled:opacity-50"
          onClick={handleCancel}
          disabled={cancelling}
          data-testid="cancel-task-btn"
        >
          {cancelling ? "Cancelling..." : "Cancel"}
        </button>
      )}
    </div>
  );
});
