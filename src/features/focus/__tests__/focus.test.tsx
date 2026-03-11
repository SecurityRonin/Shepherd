import { describe, it, expect, beforeEach, vi } from "vitest";
import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import type { Task } from "../../../types/task";
import { useStore } from "../../../store";

// Mock Monaco DiffEditor — requires browser APIs not available in jsdom
vi.mock("@monaco-editor/react", () => ({
  DiffEditor: () => <div data-testid="mock-diff-editor" />,
}));

// Mock xterm.js — requires canvas/WebGL not available in jsdom
vi.mock("@xterm/xterm", () => {
  class MockTerminal {
    open = vi.fn();
    dispose = vi.fn();
    loadAddon = vi.fn();
    onData = vi.fn();
    write = vi.fn();
    cols = 80;
    rows = 24;
  }
  return { Terminal: MockTerminal };
});

vi.mock("@xterm/addon-fit", () => {
  class MockFitAddon {
    fit = vi.fn();
    activate = vi.fn();
    dispose = vi.fn();
  }
  return { FitAddon: MockFitAddon };
});

vi.mock("@xterm/xterm/css/xterm.css", () => ({}));

// --- Test helpers ---

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

// --- SessionSidebar tests ---

describe("SessionSidebar", () => {
  beforeEach(() => {
    useStore.setState({
      tasks: {},
      focusedTaskId: null,
      pendingPermissions: [],
      viewMode: "focus",
    });
  });

  it("renders back button with Overview text", async () => {
    const { SessionSidebar } = await import("../SessionSidebar");
    render(<SessionSidebar />);
    expect(screen.getByText(/Overview/)).toBeInTheDocument();
  });

  it("renders task list with status dots", async () => {
    const task1 = makeTask({ id: 1, title: "First task", status: "running" });
    const task2 = makeTask({ id: 2, title: "Second task", status: "done" });
    useStore.setState({
      tasks: { 1: task1, 2: task2 },
    });

    const { SessionSidebar } = await import("../SessionSidebar");
    render(<SessionSidebar />);

    expect(screen.getByText("First task")).toBeInTheDocument();
    expect(screen.getByText("Second task")).toBeInTheDocument();
  });

  it("highlights active task", async () => {
    const task1 = makeTask({ id: 1, title: "Active task" });
    const task2 = makeTask({ id: 2, title: "Inactive task" });
    useStore.setState({
      tasks: { 1: task1, 2: task2 },
      focusedTaskId: 1,
    });

    const { SessionSidebar } = await import("../SessionSidebar");
    render(<SessionSidebar />);

    const activeItem = screen.getByText("Active task").closest("button");
    expect(activeItem).toHaveClass("bg-shepherd-surface");
    expect(activeItem).toHaveClass("border-l-2");
  });

  it("shows session count", async () => {
    const task1 = makeTask({ id: 1, title: "Task 1" });
    const task2 = makeTask({ id: 2, title: "Task 2" });
    const task3 = makeTask({ id: 3, title: "Task 3" });
    useStore.setState({
      tasks: { 1: task1, 2: task2, 3: task3 },
    });

    const { SessionSidebar } = await import("../SessionSidebar");
    render(<SessionSidebar />);

    expect(screen.getByText("3 sessions")).toBeInTheDocument();
  });

  it("calls exitFocus when back button clicked", async () => {
    const user = userEvent.setup();
    const { SessionSidebar } = await import("../SessionSidebar");

    useStore.setState({ viewMode: "focus", focusedTaskId: 1 });
    render(<SessionSidebar />);

    const backButton = screen.getByText(/Overview/).closest("button")!;
    await user.click(backButton);

    expect(useStore.getState().viewMode).toBe("overview");
    expect(useStore.getState().focusedTaskId).toBeNull();
  });

  it("calls enterFocus when task clicked", async () => {
    const user = userEvent.setup();
    const task1 = makeTask({ id: 1, title: "Click me" });
    const task2 = makeTask({ id: 2, title: "Other task" });
    useStore.setState({
      tasks: { 1: task1, 2: task2 },
      focusedTaskId: 2,
    });

    const { SessionSidebar } = await import("../SessionSidebar");
    render(<SessionSidebar />);

    const taskButton = screen.getByText("Click me").closest("button")!;
    await user.click(taskButton);

    expect(useStore.getState().focusedTaskId).toBe(1);
  });
});

// --- FocusView tests ---

describe("FocusView", () => {
  beforeEach(() => {
    useStore.setState({
      tasks: {},
      focusedTaskId: null,
      pendingPermissions: [],
      viewMode: "focus",
    });
  });

  it("shows 'No task selected' when no task focused", async () => {
    const { FocusView } = await import("../FocusView");
    render(<FocusView />);
    expect(screen.getByText("No task selected")).toBeInTheDocument();
  });

  it("renders task header with title and agent badge", async () => {
    const task = makeTask({ id: 1, title: "My focused task", agent_id: "claude-code" });
    useStore.setState({
      tasks: { 1: task },
      focusedTaskId: 1,
    });

    const { FocusView } = await import("../FocusView");
    render(<FocusView />);

    // Title appears in both sidebar and header — verify at least one exists
    const titleElements = screen.getAllByText("My focused task");
    expect(titleElements.length).toBeGreaterThanOrEqual(1);
    expect(screen.getByText("Claude")).toBeInTheDocument();
  });

  it("renders three panels (sidebar, terminal, diff)", async () => {
    const task = makeTask({ id: 1, title: "Paneled task" });
    useStore.setState({
      tasks: { 1: task },
      focusedTaskId: 1,
    });

    const { FocusView } = await import("../FocusView");
    render(<FocusView />);

    // SessionSidebar is present (has Sessions heading)
    expect(screen.getByText("Sessions")).toBeInTheDocument();
    // Terminal component (rendered with toolbar label)
    expect(screen.getByText("Terminal")).toBeInTheDocument();
    // DiffViewer shows empty state when no diffs
    expect(screen.getByText("No file changes yet")).toBeInTheDocument();
  });
});
