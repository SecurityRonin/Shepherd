import { describe, it, expect } from "vitest";
import type { Task } from "../../../types/task";
import {
  filterTasks,
  hasActiveFilters,
  EMPTY_FILTERS,
  type KanbanFilters,
} from "../filterTasks";

function makeTask(overrides: Partial<Task> = {}): Task {
  return {
    id: 1,
    title: "Test task",
    prompt: "Do the thing",
    agent_id: "claude-code",
    repo_path: "/repo",
    branch: "feat/test-branch",
    isolation_mode: "worktree",
    status: "running",
    created_at: new Date().toISOString(),
    updated_at: new Date().toISOString(),
    ...overrides,
  };
}

function toRecord(...tasks: Task[]): Record<number, Task> {
  const record: Record<number, Task> = {};
  for (const t of tasks) {
    record[t.id] = t;
  }
  return record;
}

describe("filterTasks", () => {
  const task1 = makeTask({ id: 1, title: "Fix login bug", agent_id: "claude-code", status: "running", branch: "fix/login", prompt: "Fix the login form" });
  const task2 = makeTask({ id: 2, title: "Add dashboard", agent_id: "codex-cli", status: "queued", branch: "feat/dashboard", prompt: "Create a new dashboard" });
  const task3 = makeTask({ id: 3, title: "Refactor API", agent_id: "aider", status: "done", branch: "refactor/api", prompt: "Refactor the REST API layer" });
  const task4 = makeTask({ id: 4, title: "Update tests", agent_id: "claude-code", status: "review", branch: "chore/tests", prompt: "Update unit tests" });

  const allTasks = toRecord(task1, task2, task3, task4);

  it("returns all tasks when no filters are active", () => {
    const result = filterTasks(allTasks, EMPTY_FILTERS);
    expect(Object.keys(result)).toHaveLength(4);
    expect(result).toEqual(allTasks);
  });

  it("filters by search term matching title (case-insensitive)", () => {
    const filters: KanbanFilters = { ...EMPTY_FILTERS, search: "fix login" };
    const result = filterTasks(allTasks, filters);
    expect(Object.keys(result)).toHaveLength(1);
    expect(result[1]).toBeDefined();
    expect(result[1].title).toBe("Fix login bug");
  });

  it("filters by search term matching branch", () => {
    const filters: KanbanFilters = { ...EMPTY_FILTERS, search: "feat/dashboard" };
    const result = filterTasks(allTasks, filters);
    expect(Object.keys(result)).toHaveLength(1);
    expect(result[2]).toBeDefined();
  });

  it("filters by search term matching prompt", () => {
    const filters: KanbanFilters = { ...EMPTY_FILTERS, search: "REST API" };
    const result = filterTasks(allTasks, filters);
    expect(Object.keys(result)).toHaveLength(1);
    expect(result[3]).toBeDefined();
  });

  it("filters by agent_id", () => {
    const filters: KanbanFilters = { ...EMPTY_FILTERS, agentId: "claude-code" };
    const result = filterTasks(allTasks, filters);
    expect(Object.keys(result)).toHaveLength(2);
    expect(result[1]).toBeDefined();
    expect(result[4]).toBeDefined();
  });

  it("filters by status", () => {
    const filters: KanbanFilters = { ...EMPTY_FILTERS, status: "queued" };
    const result = filterTasks(allTasks, filters);
    expect(Object.keys(result)).toHaveLength(1);
    expect(result[2]).toBeDefined();
  });

  it("combines search + agent_id filters", () => {
    const filters: KanbanFilters = { ...EMPTY_FILTERS, search: "test", agentId: "claude-code" };
    const result = filterTasks(allTasks, filters);
    // task4 matches both: title "Update tests" + agent "claude-code"
    // task1 has agent "claude-code" but title/branch/prompt don't contain "test"
    expect(Object.keys(result)).toHaveLength(1);
    expect(result[4]).toBeDefined();
  });

  it("combines all three filters", () => {
    const filters: KanbanFilters = { search: "update", agentId: "claude-code", status: "review" };
    const result = filterTasks(allTasks, filters);
    expect(Object.keys(result)).toHaveLength(1);
    expect(result[4]).toBeDefined();
  });

  it("returns empty when no tasks match", () => {
    const filters: KanbanFilters = { ...EMPTY_FILTERS, search: "nonexistent-term-xyz" };
    const result = filterTasks(allTasks, filters);
    expect(Object.keys(result)).toHaveLength(0);
  });

  it("returns empty record when input tasks is empty", () => {
    const result = filterTasks({}, { ...EMPTY_FILTERS, search: "anything" });
    expect(Object.keys(result)).toHaveLength(0);
  });

  it("search is case-insensitive across all fields", () => {
    const filters: KanbanFilters = { ...EMPTY_FILTERS, search: "FIX LOGIN" };
    const result = filterTasks(allTasks, filters);
    expect(Object.keys(result)).toHaveLength(1);
    expect(result[1]).toBeDefined();
  });
});

describe("hasActiveFilters", () => {
  it("returns false for EMPTY_FILTERS", () => {
    expect(hasActiveFilters(EMPTY_FILTERS)).toBe(false);
  });

  it("returns true when search is non-empty", () => {
    expect(hasActiveFilters({ ...EMPTY_FILTERS, search: "hello" })).toBe(true);
  });

  it("returns true when agentId is set", () => {
    expect(hasActiveFilters({ ...EMPTY_FILTERS, agentId: "claude-code" })).toBe(true);
  });

  it("returns true when status is set", () => {
    expect(hasActiveFilters({ ...EMPTY_FILTERS, status: "running" })).toBe(true);
  });
});
