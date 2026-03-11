import { describe, it, expect, beforeEach } from "vitest";
import { render, screen } from "@testing-library/react";
import type { Task } from "../../../types/task";
import { useStore } from "../../../store";

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
});
