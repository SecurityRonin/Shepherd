import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen, fireEvent } from "@testing-library/react";
import type { Task } from "../../../types/task";

vi.mock("../../../lib/export", async (importOriginal) => {
  const actual = await importOriginal<typeof import("../../../lib/export")>();
  return { ...actual, triggerDownload: vi.fn() };
});

function makeSampleTask(overrides: Partial<Task> = {}): Task {
  return {
    id: 1,
    title: "Fix auth bug",
    prompt: "Fix the authentication bug",
    agent_id: "claude-code",
    repo_path: "/home/user/project",
    branch: "fix/auth-bug",
    isolation_mode: "branch",
    status: "done",
    created_at: "2026-03-20T10:00:00Z",
    updated_at: "2026-03-20T10:30:00Z",
    summary: "Fixed auth bug",
    diffs: [
      {
        file_path: "src/auth.ts",
        before_content: "old",
        after_content: "new",
        language: "typescript",
      },
    ],
    ...overrides,
  };
}

const sampleMetrics = {
  total_cost_usd: 5.67,
  total_tasks: 10,
  cost_by_agent: { "claude-code": 3.5 } as Record<string, number>,
  cost_by_day: [{ date: "2026-03-20", cost: 5.67 }],
};

describe("ExportButton", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("renders export button", async () => {
    const { ExportButton } = await import("../ExportButton");
    render(<ExportButton tasks={[makeSampleTask()]} />);
    expect(screen.getByTestId("export-btn")).toBeInTheDocument();
    expect(screen.getByTestId("export-btn")).toHaveTextContent(/Export/);
  });

  it("shows dropdown menu on click", async () => {
    const { ExportButton } = await import("../ExportButton");
    render(<ExportButton tasks={[makeSampleTask()]} />);
    expect(screen.queryByTestId("export-menu")).not.toBeInTheDocument();
    fireEvent.click(screen.getByTestId("export-btn"));
    expect(screen.getByTestId("export-menu")).toBeInTheDocument();
  });

  it("hides dropdown on second click (toggle)", async () => {
    const { ExportButton } = await import("../ExportButton");
    render(<ExportButton tasks={[makeSampleTask()]} />);
    const btn = screen.getByTestId("export-btn");
    fireEvent.click(btn);
    expect(screen.getByTestId("export-menu")).toBeInTheDocument();
    fireEvent.click(btn);
    expect(screen.queryByTestId("export-menu")).not.toBeInTheDocument();
  });

  it("'Tasks as JSON' option triggers download with correct filename", async () => {
    const { triggerDownload } = await import("../../../lib/export");
    const { ExportButton } = await import("../ExportButton");
    render(<ExportButton tasks={[makeSampleTask()]} />);
    fireEvent.click(screen.getByTestId("export-btn"));
    fireEvent.click(screen.getByTestId("export-tasks-json"));
    expect(triggerDownload).toHaveBeenCalledWith(
      expect.any(String),
      expect.stringContaining("tasks"),
      "application/json"
    );
    // Verify filename ends with .json
    const callArgs = (triggerDownload as ReturnType<typeof vi.fn>).mock.calls[0];
    expect(callArgs[1]).toMatch(/\.json$/);
  });

  it("'Tasks as CSV' option triggers download with correct filename", async () => {
    const { triggerDownload } = await import("../../../lib/export");
    const { ExportButton } = await import("../ExportButton");
    render(<ExportButton tasks={[makeSampleTask()]} />);
    fireEvent.click(screen.getByTestId("export-btn"));
    fireEvent.click(screen.getByTestId("export-tasks-csv"));
    expect(triggerDownload).toHaveBeenCalledWith(
      expect.any(String),
      expect.stringContaining("tasks"),
      "text/csv"
    );
    const callArgs = (triggerDownload as ReturnType<typeof vi.fn>).mock.calls[0];
    expect(callArgs[1]).toMatch(/\.csv$/);
  });

  it("'Metrics as JSON' option shown when metrics provided", async () => {
    const { ExportButton } = await import("../ExportButton");
    render(<ExportButton tasks={[makeSampleTask()]} metrics={sampleMetrics} />);
    fireEvent.click(screen.getByTestId("export-btn"));
    expect(screen.getByTestId("export-metrics-json")).toBeInTheDocument();
  });

  it("'Metrics as JSON' option hidden when no metrics", async () => {
    const { ExportButton } = await import("../ExportButton");
    render(<ExportButton tasks={[makeSampleTask()]} />);
    fireEvent.click(screen.getByTestId("export-btn"));
    expect(screen.queryByTestId("export-metrics-json")).not.toBeInTheDocument();
  });

  it("closes dropdown after selecting an option", async () => {
    const { ExportButton } = await import("../ExportButton");
    render(<ExportButton tasks={[makeSampleTask()]} metrics={sampleMetrics} />);
    fireEvent.click(screen.getByTestId("export-btn"));
    expect(screen.getByTestId("export-menu")).toBeInTheDocument();
    fireEvent.click(screen.getByTestId("export-tasks-json"));
    expect(screen.queryByTestId("export-menu")).not.toBeInTheDocument();
  });
});
