import { describe, it, expect, beforeEach, vi } from "vitest";
import { render, screen } from "@testing-library/react";
import { useStore } from "../store";
import type { Task } from "../types/task";

// --- Mocks ---

vi.mock("../lib/ws", () => ({
  createWsClient: () => ({
    connect: vi.fn(),
    disconnect: vi.fn(),
    send: vi.fn(),
    onEvent: vi.fn(() => vi.fn()),
    onStatusChange: vi.fn(() => vi.fn()),
  }),
}));

vi.mock("@xterm/xterm", () => ({
  Terminal: class MockTerminal {
    open = vi.fn();
    write = vi.fn();
    dispose = vi.fn();
    onData = vi.fn(() => ({ dispose: vi.fn() }));
    loadAddon = vi.fn();
    cols = 80;
    rows = 24;
  },
}));

vi.mock("@xterm/addon-fit", () => ({
  FitAddon: class MockFitAddon {
    fit = vi.fn();
  },
}));

vi.mock("@xterm/addon-web-links", () => ({
  WebLinksAddon: class MockWebLinksAddon {},
}));

vi.mock("@monaco-editor/react", () => ({
  DiffEditor: () => <div data-testid="mock-diff-editor" />,
}));

vi.mock("../lib/sounds", () => ({
  playSound: vi.fn(),
  setVolume: vi.fn(),
  setSoundEnabled: vi.fn(),
  isSoundEnabled: () => true,
  getVolume: () => 0.5,
}));

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

function resetStore(): void {
  useStore.setState({
    tasks: {},
    pendingPermissions: [],
    sessions: {},
    viewMode: "overview",
    focusedTaskId: null,
    connectionStatus: "disconnected",
    isNewTaskDialogOpen: false,
    isCommandPaletteOpen: false,
    focusedPanel: "terminal",
  });
}

// --- Tests ---

describe("App integration", () => {
  beforeEach(() => {
    resetStore();
  });

  it("renders in overview mode with KanbanBoard column headers", async () => {
    const App = (await import("../App")).default;
    render(<App />);

    // KanbanBoard renders column headers
    expect(screen.getByText("Queued")).toBeInTheDocument();
    expect(screen.getByText("Running")).toBeInTheDocument();
    expect(screen.getByText("Needs Input")).toBeInTheDocument();
    expect(screen.getByText("Review")).toBeInTheDocument();
    expect(screen.getByText("Done")).toBeInTheDocument();
  });

  it("renders FocusView when viewMode is focus", async () => {
    const task = makeTask({ id: 42, title: "Focused task" });
    useStore.setState({
      tasks: { 42: task },
      viewMode: "focus",
      focusedTaskId: 42,
    });

    const App = (await import("../App")).default;
    render(<App />);

    // SessionSidebar renders task titles and "Sessions" heading
    expect(screen.getByText("Sessions")).toBeInTheDocument();
    // Task title appears in both SessionSidebar and FocusView header
    const focusedElements = screen.getAllByText("Focused task");
    expect(focusedElements.length).toBeGreaterThanOrEqual(1);
  });

  it("CommandPalette opens when isCommandPaletteOpen is true", async () => {
    useStore.setState({ isCommandPaletteOpen: true });

    const App = (await import("../App")).default;
    render(<App />);

    // CommandPalette search input placeholder
    expect(
      screen.getByPlaceholderText("Search commands..."),
    ).toBeInTheDocument();
  });

  it("NewTaskDialog opens when isNewTaskDialogOpen is true", async () => {
    useStore.setState({ isNewTaskDialogOpen: true });

    const App = (await import("../App")).default;
    render(<App />);

    // NewTaskDialog has "Task Prompt" label
    expect(screen.getByText("Task Prompt")).toBeInTheDocument();
  });

  it("adding tasks to store updates the board", async () => {
    const App = (await import("../App")).default;
    const { rerender } = render(<App />);

    // Initially no task titles
    expect(screen.queryByText("Alpha task")).not.toBeInTheDocument();

    // Add tasks to store
    useStore.setState({
      tasks: {
        1: makeTask({ id: 1, title: "Alpha task", status: "running" }),
        2: makeTask({ id: 2, title: "Beta task", status: "queued" }),
      },
    });

    rerender(<App />);

    expect(screen.getByText("Alpha task")).toBeInTheDocument();
    expect(screen.getByText("Beta task")).toBeInTheDocument();
  });

  it("enterFocus changes viewMode to focus", () => {
    const task = makeTask({ id: 10 });
    useStore.setState({ tasks: { 10: task } });

    useStore.getState().enterFocus(10);

    const state = useStore.getState();
    expect(state.viewMode).toBe("focus");
    expect(state.focusedTaskId).toBe(10);
  });
});
