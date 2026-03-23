import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";
import type { Task } from "../../types/task";

// Helper to create a sample task
function makeSampleTask(overrides: Partial<Task> = {}): Task {
  return {
    id: 1,
    title: "Fix auth bug",
    prompt: "Fix the authentication bug in login flow",
    agent_id: "claude-code",
    repo_path: "/home/user/project",
    branch: "fix/auth-bug",
    isolation_mode: "branch",
    status: "done",
    created_at: "2026-03-20T10:00:00Z",
    updated_at: "2026-03-20T10:30:00Z",
    summary: "Fixed the auth bug by updating token validation",
    diffs: [
      {
        file_path: "src/auth.ts",
        before_content: "old code",
        after_content: "new code",
        language: "typescript",
      },
      {
        file_path: "src/login.ts",
        before_content: "old login",
        after_content: "new login",
        language: "typescript",
      },
    ],
    gate_results: [
      { gate: "lint", passed: true },
      { gate: "test", passed: false },
    ],
    ...overrides,
  };
}

describe("exportTasksAsJson", () => {
  beforeEach(() => {
    vi.useFakeTimers();
    vi.setSystemTime(new Date("2026-03-23T12:00:00Z"));
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  it("returns valid JSON with exported_at and tasks array", async () => {
    const { exportTasksAsJson } = await import("../export");
    const result = exportTasksAsJson([makeSampleTask()]);
    const parsed = JSON.parse(result);
    expect(parsed).toHaveProperty("exported_at");
    expect(parsed).toHaveProperty("tasks");
    expect(Array.isArray(parsed.tasks)).toBe(true);
    expect(parsed.exported_at).toBe("2026-03-23T12:00:00.000Z");
  });

  it("maps task fields correctly", async () => {
    const { exportTasksAsJson } = await import("../export");
    const task = makeSampleTask();
    const result = exportTasksAsJson([task]);
    const parsed = JSON.parse(result);
    const exported = parsed.tasks[0];
    expect(exported.id).toBe(1);
    expect(exported.title).toBe("Fix auth bug");
    expect(exported.prompt).toBe("Fix the authentication bug in login flow");
    expect(exported.agent_id).toBe("claude-code");
    expect(exported.status).toBe("done");
    expect(exported.branch).toBe("fix/auth-bug");
    expect(exported.repo_path).toBe("/home/user/project");
    expect(exported.isolation_mode).toBe("branch");
    expect(exported.created_at).toBe("2026-03-20T10:00:00Z");
    expect(exported.updated_at).toBe("2026-03-20T10:30:00Z");
    expect(exported.summary).toBe("Fixed the auth bug by updating token validation");
    expect(exported.files_changed).toBe(2);
  });

  it("handles tasks with no diffs or summary", async () => {
    const { exportTasksAsJson } = await import("../export");
    const task = makeSampleTask({ diffs: undefined, summary: undefined });
    const result = exportTasksAsJson([task]);
    const parsed = JSON.parse(result);
    const exported = parsed.tasks[0];
    expect(exported.summary).toBe("");
    expect(exported.files_changed).toBe(0);
    expect(exported.diffs).toEqual([]);
    expect(exported.gate_results).toEqual([
      { gate: "lint", passed: true },
      { gate: "test", passed: false },
    ]);
  });

  it("includes diff file paths and gate results", async () => {
    const { exportTasksAsJson } = await import("../export");
    const task = makeSampleTask();
    const result = exportTasksAsJson([task]);
    const parsed = JSON.parse(result);
    const exported = parsed.tasks[0];
    expect(exported.diffs).toEqual([
      { file_path: "src/auth.ts", language: "typescript" },
      { file_path: "src/login.ts", language: "typescript" },
    ]);
    expect(exported.gate_results).toEqual([
      { gate: "lint", passed: true },
      { gate: "test", passed: false },
    ]);
  });
});

describe("exportTasksAsCsv", () => {
  it("returns CSV with correct headers", async () => {
    const { exportTasksAsCsv } = await import("../export");
    const result = exportTasksAsCsv([makeSampleTask()]);
    const lines = result.split("\n");
    expect(lines[0]).toBe(
      "id,title,agent_id,status,branch,repo_path,isolation_mode,created_at,updated_at,summary,files_changed"
    );
  });

  it("outputs correct row data", async () => {
    const { exportTasksAsCsv } = await import("../export");
    const task = makeSampleTask({
      title: "Simple title",
      branch: "main",
      repo_path: "/path",
      summary: "A summary",
    });
    const result = exportTasksAsCsv([task]);
    const lines = result.split("\n");
    expect(lines[1]).toBe(
      "1,Simple title,claude-code,done,main,/path,branch,2026-03-20T10:00:00Z,2026-03-20T10:30:00Z,A summary,2"
    );
  });

  it("escapes fields containing commas", async () => {
    const { exportTasksAsCsv } = await import("../export");
    const task = makeSampleTask({ title: "Fix auth, login bugs" });
    const result = exportTasksAsCsv([task]);
    const lines = result.split("\n");
    expect(lines[1]).toContain('"Fix auth, login bugs"');
  });

  it("escapes fields containing double quotes", async () => {
    const { exportTasksAsCsv } = await import("../export");
    const task = makeSampleTask({ title: 'Use "strict" mode' });
    const result = exportTasksAsCsv([task]);
    const lines = result.split("\n");
    expect(lines[1]).toContain('"Use ""strict"" mode"');
  });

  it("escapes fields containing newlines", async () => {
    const { exportTasksAsCsv } = await import("../export");
    const task = makeSampleTask({ summary: "Line one\nLine two" });
    const result = exportTasksAsCsv([task]);
    // The summary field should be quoted because it contains a newline
    expect(result).toContain('"Line one\nLine two"');
  });
});

describe("exportMetricsAsJson", () => {
  beforeEach(() => {
    vi.useFakeTimers();
    vi.setSystemTime(new Date("2026-03-23T12:00:00Z"));
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  it("returns valid JSON with metrics", async () => {
    const { exportMetricsAsJson } = await import("../export");
    const metrics = {
      total_cost_usd: 5.67,
      total_tasks: 10,
      cost_by_agent: { "claude-code": 3.5, "codex-cli": 2.17 },
      cost_by_day: [
        { date: "2026-03-20", cost: 2.0 },
        { date: "2026-03-21", cost: 3.67 },
      ],
    };
    const result = exportMetricsAsJson(metrics);
    const parsed = JSON.parse(result);
    expect(parsed.total_cost_usd).toBe(5.67);
    expect(parsed.total_tasks).toBe(10);
    expect(parsed.cost_by_agent).toEqual({ "claude-code": 3.5, "codex-cli": 2.17 });
    expect(parsed.cost_by_day).toEqual([
      { date: "2026-03-20", cost: 2.0 },
      { date: "2026-03-21", cost: 3.67 },
    ]);
  });

  it("includes exported_at timestamp", async () => {
    const { exportMetricsAsJson } = await import("../export");
    const metrics = {
      total_cost_usd: 0,
      total_tasks: 0,
      cost_by_agent: {},
      cost_by_day: [],
    };
    const result = exportMetricsAsJson(metrics);
    const parsed = JSON.parse(result);
    expect(parsed.exported_at).toBe("2026-03-23T12:00:00.000Z");
  });
});

describe("triggerDownload", () => {
  it("creates blob URL and triggers download", async () => {
    const mockClick = vi.fn();
    const mockAnchor = { click: mockClick, href: "", download: "" } as any;
    vi.spyOn(document, "createElement").mockReturnValue(mockAnchor);
    vi.spyOn(URL, "createObjectURL").mockReturnValue("blob:mock-url");
    vi.spyOn(URL, "revokeObjectURL").mockImplementation(() => {});

    const { triggerDownload } = await import("../export");
    triggerDownload('{"test": true}', "export.json", "application/json");

    expect(URL.createObjectURL).toHaveBeenCalledWith(expect.any(Blob));
    expect(mockAnchor.href).toBe("blob:mock-url");
    expect(mockAnchor.download).toBe("export.json");
    expect(mockClick).toHaveBeenCalledOnce();
    expect(URL.revokeObjectURL).toHaveBeenCalledWith("blob:mock-url");

    vi.restoreAllMocks();
  });
});
