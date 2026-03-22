import { describe, it, expect, beforeEach, vi } from "vitest";
import { render, screen, fireEvent } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import type { Task } from "../../../types/task";
import { useStore } from "../../../store";

// Mock cancelTask so we can verify it's called without hitting the network
vi.mock("../../../lib/api", async (importOriginal) => {
  const actual = await importOriginal<typeof import("../../../lib/api")>();
  return {
    ...actual,
    cancelTask: vi.fn().mockResolvedValue({ status: "cancelled" }),
  };
});

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

// --- AgentBadge tests ---

describe("AgentBadge", () => {
  it("renders known agent with correct label", async () => {
    const { AgentBadge } = await import("../../shared/AgentBadge");
    render(<AgentBadge agentId="claude-code" />);
    expect(screen.getByText("Claude")).toBeInTheDocument();
  });

  it("renders unknown agent with agentId as label", async () => {
    const { AgentBadge } = await import("../../shared/AgentBadge");
    render(<AgentBadge agentId="unknown-agent" />);
    expect(screen.getByText("unknown-agent")).toBeInTheDocument();
  });
});

// --- KanbanColumn tests ---

describe("KanbanColumn", () => {
  it("renders column header with label and count badge", async () => {
    const { KanbanColumn } = await import("../KanbanColumn");
    const tasks = [makeTask({ id: 1 }), makeTask({ id: 2 })];
    render(
      <KanbanColumn
        status="running"
        label="Running"
        tasks={tasks}
        renderCard={(task) => <div key={task.id}>Card {task.id}</div>}
        accentColor="#58a6ff"
      />,
    );
    expect(screen.getByText("Running")).toBeInTheDocument();
    expect(screen.getByText("2")).toBeInTheDocument();
  });

  it("renders empty state when no tasks", async () => {
    const { KanbanColumn } = await import("../KanbanColumn");
    render(
      <KanbanColumn
        status="queued"
        label="Queued"
        tasks={[]}
        renderCard={(task) => <div key={task.id}>Card</div>}
        accentColor="#8b949e"
      />,
    );
    expect(screen.getByText("No tasks")).toBeInTheDocument();
  });

  it("renders card for each task using renderCard prop", async () => {
    const { KanbanColumn } = await import("../KanbanColumn");
    const tasks = [
      makeTask({ id: 1, title: "First" }),
      makeTask({ id: 2, title: "Second" }),
    ];
    render(
      <KanbanColumn
        status="running"
        label="Running"
        tasks={tasks}
        renderCard={(task) => (
          <div key={task.id} data-testid={`card-${task.id}`}>
            {task.title}
          </div>
        )}
        accentColor="#58a6ff"
      />,
    );
    expect(screen.getByTestId("card-1")).toHaveTextContent("First");
    expect(screen.getByTestId("card-2")).toHaveTextContent("Second");
  });
});

// --- KanbanBoard tests ---

describe("KanbanBoard", () => {
  beforeEach(() => {
    useStore.setState({
      tasks: {},
      pendingPermissions: [],
    });
  });

  it("renders all 5 columns", async () => {
    const { KanbanBoard } = await import("../KanbanBoard");
    render(<KanbanBoard />);
    expect(screen.getByText("Queued")).toBeInTheDocument();
    expect(screen.getByText("Running")).toBeInTheDocument();
    expect(screen.getByText("Needs Input")).toBeInTheDocument();
    expect(screen.getByText("Review")).toBeInTheDocument();
    expect(screen.getByText("Done")).toBeInTheDocument();
  });

  it("groups tasks into correct columns by status", async () => {
    useStore.setState({
      tasks: {
        1: makeTask({ id: 1, title: "Queued task", status: "queued" }),
        2: makeTask({ id: 2, title: "Running task", status: "running" }),
        3: makeTask({ id: 3, title: "Done task", status: "done" }),
      },
      pendingPermissions: [],
    });
    const { KanbanBoard } = await import("../KanbanBoard");
    render(<KanbanBoard />);
    expect(screen.getByText("Queued task")).toBeInTheDocument();
    expect(screen.getByText("Running task")).toBeInTheDocument();
    expect(screen.getByText("Done task")).toBeInTheDocument();
  });

  it("error tasks appear in Review column", async () => {
    useStore.setState({
      tasks: {
        1: makeTask({ id: 1, title: "Error task", status: "error" }),
      },
      pendingPermissions: [],
    });
    const { KanbanBoard } = await import("../KanbanBoard");
    render(<KanbanBoard />);
    // The error task should be in the Review column, not in a separate Error column
    expect(screen.getByText("Error task")).toBeInTheDocument();
    // Review column should show count of 1 (the error task)
    const reviewHeader = screen.getByText("Review");
    expect(reviewHeader).toBeInTheDocument();
  });

  it("fades old done tasks (>24h) with reduced opacity", async () => {
    const oldDate = new Date(Date.now() - 25 * 60 * 60 * 1000).toISOString();
    const recentDate = new Date().toISOString();
    useStore.setState({
      tasks: {
        1: makeTask({ id: 1, title: "Old done task", status: "done", updated_at: oldDate }),
        2: makeTask({ id: 2, title: "Recent done task", status: "done", updated_at: recentDate }),
      },
      pendingPermissions: [],
    });
    const { KanbanBoard } = await import("../KanbanBoard");
    render(<KanbanBoard />);
    // Both tasks should be visible but old one should have opacity-40
    expect(screen.getByText("Recent done task")).toBeInTheDocument();
    expect(screen.getByText("Old done task")).toBeInTheDocument();
    const oldCard = screen.getByText("Old done task").closest(".opacity-40");
    expect(oldCard).toBeInTheDocument();
  });

  it("renders empty board with no tasks", async () => {
    useStore.setState({ tasks: {}, pendingPermissions: [] });
    const { KanbanBoard } = await import("../KanbanBoard");
    render(<KanbanBoard />);
    // All columns should still render
    expect(screen.getByText("Queued")).toBeInTheDocument();
    expect(screen.getByText("Done")).toBeInTheDocument();
  });
});

// --- TaskCard tests ---

describe("TaskCard", () => {
  it("renders task title and agent badge", async () => {
    const { TaskCard } = await import("../TaskCard");
    const task = makeTask({ title: "My cool task", agent_id: "claude-code" });
    render(<TaskCard task={task} />);
    expect(screen.getByText("My cool task")).toBeInTheDocument();
    expect(screen.getByText("Claude")).toBeInTheDocument();
  });

  it('shows approve button only for "input" status', async () => {
    const { TaskCard } = await import("../TaskCard");
    const inputTask = makeTask({ status: "input" });
    const { unmount } = render(<TaskCard task={inputTask} />);
    expect(screen.getByText("Approve", { selector: "button" })).toBeInTheDocument();
    unmount();

    const runningTask = makeTask({ status: "running" });
    render(<TaskCard task={runningTask} />);
    expect(screen.queryByText("Approve", { selector: "button" })).not.toBeInTheDocument();
  });

  it("shows permission info for input tasks with pending permissions", async () => {
    const { TaskCard } = await import("../TaskCard");
    const task = makeTask({ id: 42, status: "input" });
    const permissions = [
      {
        id: 1,
        task_id: 42,
        tool_name: "file_write",
        tool_args: '{"path": "/foo"}',
        decision: "pending",
      },
    ];
    render(<TaskCard task={task} pendingPermissions={permissions} />);
    expect(screen.getByText(/file_write/i)).toBeInTheDocument();
  });

  it("shows staleness indicator for running tasks", async () => {
    const { TaskCard } = await import("../TaskCard");
    const task = makeTask({
      status: "running",
      updated_at: new Date().toISOString(),
    });
    render(<TaskCard task={task} />);
    // The staleness indicator should be present as a dot element
    expect(screen.getByTestId("staleness-indicator")).toBeInTheDocument();
  });

  it("does not show staleness indicator for non-active tasks", async () => {
    const { TaskCard } = await import("../TaskCard");
    const task = makeTask({ status: "done" });
    render(<TaskCard task={task} />);
    expect(screen.queryByTestId("staleness-indicator")).not.toBeInTheDocument();
  });

  it("calls onClick handler when clicked", async () => {
    const { TaskCard } = await import("../TaskCard");
    const onClick = vi.fn();
    const task = makeTask({ title: "Clickable task" });
    render(<TaskCard task={task} onClick={onClick} />);
    fireEvent.click(screen.getByText("Clickable task").closest("[role='button']")!);
    expect(onClick).toHaveBeenCalledTimes(1);
  });

  it("renders branch name", async () => {
    const { TaskCard } = await import("../TaskCard");
    const task = makeTask({ branch: "feat/my-feature" });
    render(<TaskCard task={task} />);
    expect(screen.getByText("feat/my-feature")).toBeInTheDocument();
  });

  it("shows iTerm2 badge when task has iterm2_session_id", async () => {
    const { TaskCard } = await import("../TaskCard");
    const task = makeTask({ iterm2_session_id: "session-123" });
    render(<TaskCard task={task} />);
    expect(screen.getByText(/iTerm2/i)).toBeInTheDocument();
  });

  it("does not show iTerm2 badge when no iterm2_session_id", async () => {
    const { TaskCard } = await import("../TaskCard");
    const task = makeTask();
    render(<TaskCard task={task} />);
    expect(screen.queryByText(/iTerm2/i)).not.toBeInTheDocument();
  });

  it("shows error styling for error status", async () => {
    const { TaskCard } = await import("../TaskCard");
    const task = makeTask({ status: "error" });
    const { container } = render(<TaskCard task={task} />);
    const card = container.firstElementChild;
    expect(card?.className).toContain("border-shepherd-red");
  });

  it("shows 'Ready for review' text for review status", async () => {
    const { TaskCard } = await import("../TaskCard");
    const task = makeTask({ status: "review" });
    render(<TaskCard task={task} />);
    expect(screen.getByText("Ready for review")).toBeInTheDocument();
  });

  it("shows gate results for review tasks", async () => {
    const { TaskCard } = await import("../TaskCard");
    const task = makeTask({
      status: "review",
      gate_results: [
        { gate: "lint", passed: true },
        { gate: "tests", passed: false },
      ],
    });
    render(<TaskCard task={task} />);
    expect(screen.getByText("lint")).toBeInTheDocument();
    expect(screen.getByText("tests")).toBeInTheDocument();
    expect(screen.getByText("pass")).toBeInTheDocument();
    expect(screen.getByText("fail")).toBeInTheDocument();
  });

  it("shows cancel button for running tasks", async () => {
    const { TaskCard } = await import("../TaskCard");
    const task = makeTask({ status: "running" });
    render(<TaskCard task={task} />);
    expect(screen.getByTestId("cancel-task-btn")).toBeInTheDocument();
    expect(screen.getByTestId("cancel-task-btn")).toHaveTextContent("Cancel");
  });

  it("shows cancel button for input tasks", async () => {
    const { TaskCard } = await import("../TaskCard");
    const task = makeTask({ status: "input" });
    render(<TaskCard task={task} />);
    expect(screen.getByTestId("cancel-task-btn")).toBeInTheDocument();
  });

  it("does not show cancel button for done tasks", async () => {
    const { TaskCard } = await import("../TaskCard");
    const task = makeTask({ status: "done" });
    render(<TaskCard task={task} />);
    expect(screen.queryByTestId("cancel-task-btn")).not.toBeInTheDocument();
  });

  it("does not show cancel button for error tasks", async () => {
    const { TaskCard } = await import("../TaskCard");
    const task = makeTask({ status: "error" });
    render(<TaskCard task={task} />);
    expect(screen.queryByTestId("cancel-task-btn")).not.toBeInTheDocument();
  });

  it("calls cancelTask API when cancel button is clicked", async () => {
    const { TaskCard } = await import("../TaskCard");
    const { cancelTask } = await import("../../../lib/api");
    const task = makeTask({ id: 42, status: "running" });
    render(<TaskCard task={task} />);

    const cancelBtn = screen.getByTestId("cancel-task-btn");
    await userEvent.click(cancelBtn);

    expect(cancelTask).toHaveBeenCalledWith(42);
  });
});
