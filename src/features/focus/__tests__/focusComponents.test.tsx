import { describe, it, expect, beforeEach, vi } from "vitest";
import { render, screen } from "@testing-library/react";
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
});
