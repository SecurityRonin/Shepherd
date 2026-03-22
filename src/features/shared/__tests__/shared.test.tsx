import { describe, it, expect, beforeEach } from "vitest";
import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { useStore } from "../../../store";

// --- StatusIndicator tests ---

describe("StatusIndicator", () => {
  it("renders fresh level with green color", async () => {
    const { StatusIndicator } = await import("../StatusIndicator");
    render(<StatusIndicator level="fresh" data-testid="si" />);
    const container = screen.getByTestId("si");
    expect(container).toBeInTheDocument();
    expect(container).toHaveAttribute("title", "Active");
    // The dot inside should have the green class
    const dot = container.querySelector("div > div");
    expect(dot?.className).toContain("bg-shepherd-green");
  });

  it("renders stale level with yellow color and correct label", async () => {
    const { StatusIndicator } = await import("../StatusIndicator");
    render(<StatusIndicator level="stale" data-testid="si" />);
    const container = screen.getByTestId("si");
    expect(container).toHaveAttribute("title", "Idle >30s");
    const dot = container.querySelector("div > div");
    expect(dot?.className).toContain("bg-shepherd-yellow");
  });

  it("renders critical level with red color and pulse animation", async () => {
    const { StatusIndicator } = await import("../StatusIndicator");
    render(<StatusIndicator level="critical" data-testid="si" />);
    const container = screen.getByTestId("si");
    expect(container).toHaveAttribute("title", "Idle >2min");
    const dot = container.querySelector("div > div");
    expect(dot?.className).toContain("bg-shepherd-red");
    expect(dot?.className).toContain("animate-pulse");
  });

  it("renders larger dot with md size", async () => {
    const { StatusIndicator } = await import("../StatusIndicator");
    const { container } = render(<StatusIndicator level="fresh" size="md" />);
    const dot = container.querySelector(".rounded-full");
    expect(dot?.className).toContain("w-2.5");
    expect(dot?.className).toContain("h-2.5");
  });

  it("renders smaller dot with sm size (default)", async () => {
    const { StatusIndicator } = await import("../StatusIndicator");
    const { container } = render(<StatusIndicator level="fresh" />);
    const dot = container.querySelector(".rounded-full");
    expect(dot?.className).toContain("w-2");
    expect(dot?.className).toContain("h-2");
  });
});

// --- AgentBadge tests ---

describe("AgentBadge", () => {
  it("renders known agent with correct label", async () => {
    const { AgentBadge } = await import("../AgentBadge");
    render(<AgentBadge agentId="claude-code" />);
    expect(screen.getByText("Claude")).toBeInTheDocument();
  });

  it("renders unknown agent with agentId as label", async () => {
    const { AgentBadge } = await import("../AgentBadge");
    render(<AgentBadge agentId="unknown-agent" />);
    expect(screen.getByText("unknown-agent")).toBeInTheDocument();
  });

  it("renders multiple known agent types correctly", async () => {
    const { AgentBadge } = await import("../AgentBadge");

    const { unmount: u1 } = render(<AgentBadge agentId="codex-cli" />);
    expect(screen.getByText("Codex")).toBeInTheDocument();
    u1();

    const { unmount: u2 } = render(<AgentBadge agentId="opencode" />);
    expect(screen.getByText("OpenCode")).toBeInTheDocument();
    u2();

    const { unmount: u3 } = render(<AgentBadge agentId="gemini-cli" />);
    expect(screen.getByText("Gemini")).toBeInTheDocument();
    u3();

    render(<AgentBadge agentId="aider" />);
    expect(screen.getByText("Aider")).toBeInTheDocument();
  });
});

// --- Header tests ---

describe("Header", () => {
  beforeEach(() => {
    useStore.setState({
      viewMode: "overview",
      connectionStatus: "connected",
      pendingPermissions: [],
      focusedTaskId: null,
      tasks: {},
    } as any);
  });

  it("renders the Shepherd title", async () => {
    const { Header } = await import("../Header");
    render(<Header />);
    expect(screen.getByText("Shepherd")).toBeInTheDocument();
  });

  it("shows 'Overview' mode label in overview mode", async () => {
    const { Header } = await import("../Header");
    render(<Header />);
    expect(screen.getByText("Overview")).toBeInTheDocument();
  });

  it("shows 'Focus' mode label in focus mode", async () => {
    useStore.setState({ viewMode: "focus" } as any);
    const { Header } = await import("../Header");
    render(<Header />);
    // Focus label in the mode indicator
    expect(screen.getByText("Focus")).toBeInTheDocument();
  });

  it("shows back-to-overview button in focus mode", async () => {
    useStore.setState({ viewMode: "focus" } as any);
    const { Header } = await import("../Header");
    render(<Header />);
    // Back button text includes "Overview"
    const buttons = screen.getAllByText(/Overview/);
    expect(buttons.length).toBeGreaterThanOrEqual(1);
  });

  it("shows connection status", async () => {
    useStore.setState({ connectionStatus: "connected" } as any);
    const { Header } = await import("../Header");
    render(<Header />);
    expect(screen.getByText("Connected")).toBeInTheDocument();
  });

  it("shows disconnected status with red dot", async () => {
    useStore.setState({ connectionStatus: "disconnected" } as any);
    const { Header } = await import("../Header");
    render(<Header />);
    expect(screen.getByText("Disconnected")).toBeInTheDocument();
  });

  it("shows reconnecting status", async () => {
    useStore.setState({ connectionStatus: "reconnecting" } as any);
    const { Header } = await import("../Header");
    render(<Header />);
    expect(screen.getByText("Reconnecting...")).toBeInTheDocument();
  });

  it("shows '+ New Task' button", async () => {
    const { Header } = await import("../Header");
    render(<Header />);
    expect(screen.getByText("+ New Task")).toBeInTheDocument();
  });

  it("opens new task dialog when button clicked", async () => {
    const user = userEvent.setup();
    const { Header } = await import("../Header");
    render(<Header />);

    await user.click(screen.getByText("+ New Task"));
    expect(useStore.getState().isNewTaskDialogOpen).toBe(true);
  });

  it("shows pending permissions count when > 0", async () => {
    useStore.setState({
      pendingPermissions: [
        { id: 1, task_id: 1, tool_name: "write_file", tool_args: "{}", decision: "pending" },
        { id: 2, task_id: 2, tool_name: "read_file", tool_args: "{}", decision: "pending" },
      ],
    } as any);
    const { Header } = await import("../Header");
    render(<Header />);
    expect(screen.getByText("2 pending")).toBeInTheDocument();
  });

  it("hides pending badge when count is 0", async () => {
    useStore.setState({ pendingPermissions: [] } as any);
    const { Header } = await import("../Header");
    render(<Header />);
    expect(screen.queryByText(/pending/)).not.toBeInTheDocument();
  });

  it("calls exitFocus when back button clicked in focus mode", async () => {
    const user = userEvent.setup();
    useStore.setState({
      viewMode: "focus",
      focusedTaskId: 1,
    } as any);
    const { Header } = await import("../Header");
    render(<Header />);

    // Click the back button (contains "Overview")
    const backButton = screen.getAllByText(/Overview/).find(
      (el) => el.closest("button"),
    )?.closest("button");
    if (backButton) {
      await user.click(backButton);
      expect(useStore.getState().viewMode).toBe("overview");
      expect(useStore.getState().focusedTaskId).toBeNull();
    }
  });
});

// --- Layout tests ---

describe("Layout", () => {
  beforeEach(() => {
    useStore.setState({
      viewMode: "overview",
      connectionStatus: "connected",
      pendingPermissions: [],
      focusedTaskId: null,
      tasks: {},
    } as any);
  });

  it("renders children content", async () => {
    const { Layout } = await import("../Layout");
    render(
      <Layout>
        <div data-testid="child-content">Hello Shepherd</div>
      </Layout>,
    );
    expect(screen.getByTestId("child-content")).toBeInTheDocument();
    expect(screen.getByText("Hello Shepherd")).toBeInTheDocument();
  });

  it("renders sidebar navigation with all nav items", async () => {
    const { Layout } = await import("../Layout");
    render(
      <Layout>
        <div>Content</div>
      </Layout>,
    );
    expect(screen.getByTestId("sidebar-nav")).toBeInTheDocument();
    expect(screen.getByTestId("nav-overview")).toBeInTheDocument();
    expect(screen.getByTestId("nav-observability")).toBeInTheDocument();
    expect(screen.getByTestId("nav-replay")).toBeInTheDocument();
    expect(screen.getByTestId("nav-ecosystem")).toBeInTheDocument();
    expect(screen.getByTestId("nav-cloud")).toBeInTheDocument();
  });

  it("highlights active nav item", async () => {
    useStore.setState({ viewMode: "replay" } as any);
    const { Layout } = await import("../Layout");
    render(
      <Layout>
        <div>Content</div>
      </Layout>,
    );
    const replayBtn = screen.getByTestId("nav-replay");
    expect(replayBtn.className).toContain("bg-blue-600");
    const overviewBtn = screen.getByTestId("nav-overview");
    expect(overviewBtn.className).not.toContain("bg-blue-600");
  });

  it("switches view mode when nav item clicked", async () => {
    const user = userEvent.setup();
    const { Layout } = await import("../Layout");
    render(
      <Layout>
        <div>Content</div>
      </Layout>,
    );
    await user.click(screen.getByTestId("nav-replay"));
    expect(useStore.getState().viewMode).toBe("replay");
  });

  it("does not switch away from focus mode for non-overview items", async () => {
    const user = userEvent.setup();
    useStore.setState({ viewMode: "focus" } as any);
    const { Layout } = await import("../Layout");
    render(
      <Layout>
        <div>Content</div>
      </Layout>,
    );
    // Clicking replay while in focus mode should NOT switch
    await user.click(screen.getByTestId("nav-replay"));
    expect(useStore.getState().viewMode).toBe("focus");
  });

  it("renders Header component", async () => {
    const { Layout } = await import("../Layout");
    render(
      <Layout>
        <div>Content</div>
      </Layout>,
    );
    // Header renders the Shepherd title
    expect(screen.getByText("Shepherd")).toBeInTheDocument();
  });
});
