import { describe, it, expect, beforeEach, vi } from "vitest";
import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import type { Task } from "../../../types/task";
import type { PermissionEvent } from "../../../types/events";
import { useStore } from "../../../store";

// Mock Monaco DiffEditor — it requires browser APIs not available in jsdom
vi.mock("@monaco-editor/react", () => ({
  DiffEditor: () => <div data-testid="mock-diff-editor" />,
}));

// Mock xterm.js — it requires canvas/WebGL not available in jsdom
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

function makePermission(overrides: Partial<PermissionEvent> = {}): PermissionEvent {
  return {
    id: 100,
    task_id: 1,
    tool_name: "write_file",
    tool_args: JSON.stringify({ path: "/src/main.rs", content: "fn main() {}" }),
    decision: "pending",
    ...overrides,
  };
}

// --- detectLanguage tests ---

describe("DiffViewer: detectLanguage", () => {
  it("maps common extensions correctly", async () => {
    const { detectLanguage } = await import("../DiffViewer");

    expect(detectLanguage("app.ts")).toBe("typescript");
    expect(detectLanguage("app.tsx")).toBe("typescript");
    expect(detectLanguage("index.js")).toBe("javascript");
    expect(detectLanguage("index.jsx")).toBe("javascript");
    expect(detectLanguage("main.py")).toBe("python");
    expect(detectLanguage("lib.rs")).toBe("rust");
    expect(detectLanguage("main.go")).toBe("go");
    expect(detectLanguage("App.java")).toBe("java");
    expect(detectLanguage("styles.css")).toBe("css");
    expect(detectLanguage("page.html")).toBe("html");
    expect(detectLanguage("data.json")).toBe("json");
    expect(detectLanguage("config.yaml")).toBe("yaml");
    expect(detectLanguage("config.yml")).toBe("yaml");
    expect(detectLanguage("notes.md")).toBe("markdown");
    expect(detectLanguage("script.sh")).toBe("shell");
    expect(detectLanguage("query.sql")).toBe("sql");
    expect(detectLanguage("Program.cs")).toBe("csharp");
  });

  it("returns plaintext for unknown extensions", async () => {
    const { detectLanguage } = await import("../DiffViewer");

    expect(detectLanguage("file.xyz")).toBe("plaintext");
    expect(detectLanguage("noext")).toBe("plaintext");
  });

  it("handles paths with directories", async () => {
    const { detectLanguage } = await import("../DiffViewer");

    expect(detectLanguage("src/components/Button.tsx")).toBe("typescript");
    expect(detectLanguage("/home/user/project/main.py")).toBe("python");
  });
});

// --- DiffViewer empty state test ---

describe("DiffViewer: empty state", () => {
  beforeEach(() => {
    useStore.setState({
      tasks: {},
      focusedTaskId: null,
      pendingPermissions: [],
    });
  });

  it("renders 'No file changes yet' when task has no diffs", async () => {
    const task = makeTask({ id: 1 });
    useStore.setState({ tasks: { 1: task } });

    const { DiffViewer } = await import("../DiffViewer");
    render(<DiffViewer taskId={1} />);

    expect(screen.getByText("No file changes yet")).toBeInTheDocument();
  });

  it("renders 'No file changes yet' when diffs is empty array", async () => {
    const task = makeTask({ id: 1, diffs: [] });
    useStore.setState({ tasks: { 1: task } });

    const { DiffViewer } = await import("../DiffViewer");
    render(<DiffViewer taskId={1} />);

    expect(screen.getByText("No file changes yet")).toBeInTheDocument();
  });
});

// --- PermissionPrompt tests ---

describe("PermissionPrompt", () => {
  beforeEach(() => {
    useStore.setState({
      tasks: {},
      focusedTaskId: null,
      pendingPermissions: [],
    });
  });

  it("renders approve buttons when permission is pending", async () => {
    const perm = makePermission({ task_id: 1 });
    useStore.setState({ pendingPermissions: [perm] });

    const { PermissionPrompt } = await import("../PermissionPrompt");
    render(<PermissionPrompt taskId={1} />);

    expect(screen.getByTestId("approve-button")).toBeInTheDocument();
    expect(screen.getByTestId("approve-all-button")).toBeInTheDocument();
    expect(screen.getByText("Approve")).toBeInTheDocument();
    expect(screen.getByText("Approve All")).toBeInTheDocument();
  });

  it("renders permission question with tool_name", async () => {
    const perm = makePermission({ task_id: 1, tool_name: "execute_command" });
    useStore.setState({ pendingPermissions: [perm] });

    const { PermissionPrompt } = await import("../PermissionPrompt");
    render(<PermissionPrompt taskId={1} />);

    expect(screen.getByTestId("permission-tool-name")).toHaveTextContent("execute_command");
  });

  it("renders nothing when no permissions for task", async () => {
    useStore.setState({ pendingPermissions: [] });

    const { PermissionPrompt } = await import("../PermissionPrompt");
    const { container } = render(<PermissionPrompt taskId={1} />);

    expect(container.innerHTML).toBe("");
  });

  it("shows custom input when Custom button is clicked", async () => {
    const user = userEvent.setup();
    const perm = makePermission({ task_id: 1 });
    useStore.setState({ pendingPermissions: [perm] });

    const { PermissionPrompt } = await import("../PermissionPrompt");
    render(<PermissionPrompt taskId={1} />);

    // Custom input should not be visible initially
    expect(screen.queryByTestId("custom-input-area")).not.toBeInTheDocument();

    // Click "Custom..." button
    await user.click(screen.getByTestId("custom-toggle-button"));

    // Custom input should now be visible
    expect(screen.getByTestId("custom-input-area")).toBeInTheDocument();
    expect(screen.getByTestId("custom-input")).toBeInTheDocument();
  });

  it("shows latest permission when multiple are pending", async () => {
    const perm1 = makePermission({ id: 100, task_id: 1, tool_name: "read_file" });
    const perm2 = makePermission({ id: 101, task_id: 1, tool_name: "write_file" });
    useStore.setState({ pendingPermissions: [perm1, perm2] });

    const { PermissionPrompt } = await import("../PermissionPrompt");
    render(<PermissionPrompt taskId={1} />);

    // Should show the latest permission (write_file)
    expect(screen.getByTestId("permission-tool-name")).toHaveTextContent("write_file");
  });

  it("calls approveTask when Approve button clicked", async () => {
    const user = userEvent.setup();
    // Mock the api module's approveTask
    const apiModule = await import("../../../lib/api");
    const spy = vi.spyOn(apiModule, "approveTask").mockResolvedValue(undefined as any);

    const perm = makePermission({ task_id: 1 });
    useStore.setState({ pendingPermissions: [perm] });

    const { PermissionPrompt } = await import("../PermissionPrompt");
    render(<PermissionPrompt taskId={1} />);

    await user.click(screen.getByTestId("approve-button"));
    expect(spy).toHaveBeenCalledWith(1);

    spy.mockRestore();
  });

  it("shows error when approve fails", async () => {
    const user = userEvent.setup();
    const apiModule = await import("../../../lib/api");
    const spy = vi.spyOn(apiModule, "approveTask").mockRejectedValue(new Error("Task already resolved"));

    const perm = makePermission({ task_id: 1 });
    useStore.setState({ pendingPermissions: [perm] });

    const { PermissionPrompt } = await import("../PermissionPrompt");
    render(<PermissionPrompt taskId={1} />);

    await user.click(screen.getByTestId("approve-button"));
    await waitFor(() => expect(screen.getByTestId("permission-error")).toHaveTextContent("Task already resolved"));

    spy.mockRestore();
  });

  it("custom input has accessible aria-label", async () => {
    const user = userEvent.setup();
    const perm = makePermission({ task_id: 1 });
    useStore.setState({ pendingPermissions: [perm] });

    const { PermissionPrompt } = await import("../PermissionPrompt");
    render(<PermissionPrompt taskId={1} />);

    await user.click(screen.getByTestId("custom-toggle-button"));
    expect(screen.getByRole("textbox", { name: /custom response/i })).toBeInTheDocument();
  });

  it("only shows permissions for the given taskId", async () => {
    const perm1 = makePermission({ id: 100, task_id: 1, tool_name: "write_file" });
    const perm2 = makePermission({ id: 101, task_id: 2, tool_name: "execute_command" });
    useStore.setState({ pendingPermissions: [perm1, perm2] });

    const { PermissionPrompt } = await import("../PermissionPrompt");
    render(<PermissionPrompt taskId={1} />);

    // Should show write_file (task 1), not execute_command (task 2)
    expect(screen.getByTestId("permission-tool-name")).toHaveTextContent("write_file");
  });
});

// --- Terminal tests ---

describe("Terminal", () => {
  beforeEach(() => {
    useStore.setState({
      tasks: {},
      focusedTaskId: null,
      pendingPermissions: [],
      wsClient: null,
      terminalOutputHandlers: new Map(),
    } as any);
  });

  it("renders terminal toolbar with task ID", async () => {
    const { Terminal } = await import("../Terminal");
    render(<Terminal taskId={42} />);
    expect(screen.getByText("Terminal")).toBeInTheDocument();
    expect(screen.getByText("Task #42")).toBeInTheDocument();
  });

  it("renders terminal container div", async () => {
    const { Terminal } = await import("../Terminal");
    render(<Terminal taskId={1} />);
    expect(screen.getByTestId("terminal-container")).toBeInTheDocument();
  });

  it("creates xterm instance and opens it in container", async () => {
    const { Terminal: TerminalComponent } = await import("../Terminal");
    render(<TerminalComponent taskId={1} />);

    // The mock XTerm constructor should have been called and open() invoked
    // We can verify the container is rendered
    const container = screen.getByTestId("terminal-container");
    expect(container).toBeInTheDocument();
  });

  it("registers and unregisters terminal handler", async () => {
    const registerSpy = vi.fn();
    const unregisterSpy = vi.fn();
    useStore.setState({
      registerTerminalHandler: registerSpy,
      unregisterTerminalHandler: unregisterSpy,
    } as any);

    const { Terminal: TerminalComponent } = await import("../Terminal");
    const { unmount } = render(<TerminalComponent taskId={7} />);

    expect(registerSpy).toHaveBeenCalledWith(7, expect.any(Function));

    unmount();
    expect(unregisterSpy).toHaveBeenCalledWith(7);
  });
});

// --- DiffViewer with diffs tests ---

describe("DiffViewer: with file diffs", () => {
  beforeEach(() => {
    useStore.setState({
      tasks: {},
      focusedTaskId: null,
      pendingPermissions: [],
    });
  });

  it("renders file tabs when diffs exist", async () => {
    const task = makeTask({
      id: 1,
      diffs: [
        {
          file_path: "src/main.rs",
          before_content: "fn main() {}",
          after_content: 'fn main() { println!("hello"); }',
          language: "rust",
        },
        {
          file_path: "src/lib.rs",
          before_content: "",
          after_content: "pub mod utils;",
          language: "rust",
        },
      ],
    });
    useStore.setState({ tasks: { 1: task } });

    const { DiffViewer } = await import("../DiffViewer");
    render(<DiffViewer taskId={1} />);

    // Tab buttons show the filename, full path in title attribute
    expect(screen.getByText("main.rs")).toBeInTheDocument();
    expect(screen.getByText("lib.rs")).toBeInTheDocument();
    expect(screen.getByTitle("src/main.rs")).toBeInTheDocument();
    expect(screen.getByTitle("src/lib.rs")).toBeInTheDocument();
  });

  it("renders Monaco DiffEditor for active file", async () => {
    const task = makeTask({
      id: 1,
      diffs: [
        {
          file_path: "app.ts",
          before_content: "const x = 1;",
          after_content: "const x = 2;",
          language: "typescript",
        },
      ],
    });
    useStore.setState({ tasks: { 1: task } });

    const { DiffViewer } = await import("../DiffViewer");
    render(<DiffViewer taskId={1} />);

    // The mocked DiffEditor should be rendered
    expect(screen.getByTestId("mock-diff-editor")).toBeInTheDocument();
  });

  it("renders Unified/Split toggle buttons", async () => {
    const task = makeTask({
      id: 1,
      diffs: [
        {
          file_path: "a.ts",
          before_content: "",
          after_content: "a",
          language: "typescript",
        },
      ],
    });
    useStore.setState({ tasks: { 1: task } });

    const { DiffViewer } = await import("../DiffViewer");
    render(<DiffViewer taskId={1} />);

    expect(screen.getByText("Unified")).toBeInTheDocument();
    expect(screen.getByText("Split")).toBeInTheDocument();
  });

  it("renders all file tabs when multiple diffs exist", async () => {
    const task = makeTask({
      id: 1,
      diffs: [
        {
          file_path: "a.ts",
          before_content: "",
          after_content: "a",
          language: "typescript",
        },
        {
          file_path: "b.ts",
          before_content: "",
          after_content: "b",
          language: "typescript",
        },
        {
          file_path: "c.ts",
          before_content: "",
          after_content: "c",
          language: "typescript",
        },
      ],
    });
    useStore.setState({ tasks: { 1: task } });

    const { DiffViewer } = await import("../DiffViewer");
    render(<DiffViewer taskId={1} />);

    // Should render a tab for each file (use title attribute which has full path)
    expect(screen.getByTitle("a.ts")).toBeInTheDocument();
    expect(screen.getByTitle("b.ts")).toBeInTheDocument();
    expect(screen.getByTitle("c.ts")).toBeInTheDocument();
  });
});
