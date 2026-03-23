import type { Task, TaskStatus } from "../../types/task";

export interface KanbanFilters {
  search: string;
  agentId: string | null;
  status: TaskStatus | null;
}

export const EMPTY_FILTERS: KanbanFilters = {
  search: "",
  agentId: null,
  status: null,
};

export function filterTasks(
  tasks: Record<number, Task>,
  filters: KanbanFilters,
): Record<number, Task> {
  if (!hasActiveFilters(filters)) {
    return tasks;
  }

  const searchLower = filters.search.toLowerCase();
  const result: Record<number, Task> = {};

  for (const [id, task] of Object.entries(tasks)) {
    // Filter by agent_id (exact match)
    if (filters.agentId !== null && task.agent_id !== filters.agentId) {
      continue;
    }

    // Filter by status (exact match)
    if (filters.status !== null && task.status !== filters.status) {
      continue;
    }

    // Filter by search (case-insensitive match on title, branch, prompt)
    if (searchLower !== "") {
      const matchesSearch =
        task.title.toLowerCase().includes(searchLower) ||
        task.branch.toLowerCase().includes(searchLower) ||
        task.prompt.toLowerCase().includes(searchLower);

      if (!matchesSearch) {
        continue;
      }
    }

    result[Number(id)] = task;
  }

  return result;
}

export function hasActiveFilters(filters: KanbanFilters): boolean {
  return (
    filters.search !== "" ||
    filters.agentId !== null ||
    filters.status !== null
  );
}
