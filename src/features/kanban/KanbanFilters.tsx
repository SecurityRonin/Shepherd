import React from "react";
import { AGENT_COLORS } from "../../types/task";
import {
  EMPTY_FILTERS,
  hasActiveFilters,
  type KanbanFilters as KanbanFiltersType,
} from "./filterTasks";
import type { TaskStatus } from "../../types/task";

const STATUS_LABELS: Record<TaskStatus, string> = {
  queued: "queued",
  running: "running",
  input: "input",
  review: "review",
  error: "error",
  done: "done",
  cancelled: "cancelled",
};

const ALL_STATUSES: TaskStatus[] = [
  "queued",
  "running",
  "input",
  "review",
  "error",
  "done",
  "cancelled",
];

interface KanbanFiltersProps {
  filters: KanbanFiltersType;
  onFiltersChange: (filters: KanbanFiltersType) => void;
}

export const KanbanFilters: React.FC<KanbanFiltersProps> = ({
  filters,
  onFiltersChange,
}) => {
  const filtersActive = hasActiveFilters(filters);

  return (
    <div className="flex items-center gap-3 px-4 pt-4 pb-1">
      <input
        type="text"
        placeholder="Search tasks..."
        value={filters.search}
        onChange={(e) =>
          onFiltersChange({ ...filters, search: e.target.value })
        }
        className="flex-1 max-w-xs px-3 py-1.5 text-sm rounded bg-shepherd-bg border border-shepherd-border text-shepherd-text placeholder:text-shepherd-muted focus:outline-none focus:border-shepherd-accent"
      />

      <select
        aria-label="Agent filter"
        value={filters.agentId ?? ""}
        onChange={(e) =>
          onFiltersChange({
            ...filters,
            agentId: e.target.value || null,
          })
        }
        className="px-3 py-1.5 text-sm rounded bg-shepherd-bg border border-shepherd-border text-shepherd-text focus:outline-none focus:border-shepherd-accent"
      >
        <option value="">All Agents</option>
        {Object.values(AGENT_COLORS).map((agent) => (
          <option key={agent.id} value={agent.id}>
            {agent.label}
          </option>
        ))}
      </select>

      <select
        aria-label="Status filter"
        value={filters.status ?? ""}
        onChange={(e) =>
          onFiltersChange({
            ...filters,
            status: (e.target.value as TaskStatus) || null,
          })
        }
        className="px-3 py-1.5 text-sm rounded bg-shepherd-bg border border-shepherd-border text-shepherd-text focus:outline-none focus:border-shepherd-accent"
      >
        <option value="">All Statuses</option>
        {ALL_STATUSES.map((s) => (
          <option key={s} value={s}>
            {STATUS_LABELS[s]}
          </option>
        ))}
      </select>

      {filtersActive && (
        <button
          onClick={() => onFiltersChange(EMPTY_FILTERS)}
          className="px-3 py-1.5 text-sm rounded bg-shepherd-border hover:bg-shepherd-muted/30 text-shepherd-text transition-colors"
        >
          Clear
        </button>
      )}
    </div>
  );
};
