import { describe, it, expect, vi, beforeEach } from "vitest";
import { renderHook } from "@testing-library/react";
import { useStore } from "../../store";

// Mock the keys module
const mockRegister = vi.fn();
const mockHandleKeyDown = vi.fn();
vi.mock("../../lib/keys", () => ({
  createShortcutManager: () => ({
    register: mockRegister,
    handleKeyDown: mockHandleKeyDown,
    shortcuts: [],
  }),
}));

beforeEach(() => {
  mockRegister.mockClear();
  mockHandleKeyDown.mockClear();
  useStore.setState({
    viewMode: "overview",
    focusedTaskId: null,
    isCommandPaletteOpen: false,
  } as any);
});

describe("useKeyboardShortcuts", () => {
  it("registers all expected shortcuts on mount", async () => {
    const { useKeyboardShortcuts } = await import("../useKeyboardShortcuts");
    const wsRef = { current: null };
    renderHook(() => useKeyboardShortcuts(wsRef as any));

    // Core shortcuts: toggle-view, new-task, approve-current, approve-all,
    // focus-terminal, focus-changes, command-palette, + 9 quick-approve = 16 total
    expect(mockRegister).toHaveBeenCalledTimes(16);

    const ids = mockRegister.mock.calls.map((c: any) => c[0].id);
    expect(ids).toContain("toggle-view");
    expect(ids).toContain("new-task");
    expect(ids).toContain("approve-current");
    expect(ids).toContain("approve-all");
    expect(ids).toContain("command-palette");
    expect(ids).toContain("focus-terminal");
    expect(ids).toContain("focus-changes");
    expect(ids).toContain("quick-approve-1");
    expect(ids).toContain("quick-approve-9");
  });

  it("registers meta+0 for toggle-view", async () => {
    const { useKeyboardShortcuts } = await import("../useKeyboardShortcuts");
    const wsRef = { current: null };
    renderHook(() => useKeyboardShortcuts(wsRef as any));

    const toggleView = mockRegister.mock.calls.find((c: any) => c[0].id === "toggle-view");
    expect(toggleView).toBeDefined();
    expect(toggleView![0].keys).toBe("meta+0");
  });

  it("toggle-view handler calls store.toggleView", async () => {
    const { useKeyboardShortcuts } = await import("../useKeyboardShortcuts");
    const wsRef = { current: null };
    renderHook(() => useKeyboardShortcuts(wsRef as any));

    const toggleView = mockRegister.mock.calls.find((c: any) => c[0].id === "toggle-view");
    const toggleSpy = vi.spyOn(useStore.getState(), "toggleView");
    toggleView![0].handler();
    expect(toggleSpy).toHaveBeenCalled();
  });

  it("new-task handler opens new task dialog", async () => {
    const { useKeyboardShortcuts } = await import("../useKeyboardShortcuts");
    const wsRef = { current: null };
    renderHook(() => useKeyboardShortcuts(wsRef as any));

    const newTask = mockRegister.mock.calls.find((c: any) => c[0].id === "new-task");
    newTask![0].handler();
    expect(useStore.getState().isNewTaskDialogOpen).toBe(true);
  });

  it("approve-current sends task_approve via WebSocket", async () => {
    useStore.setState({ focusedTaskId: 42 } as any);
    const mockSend = vi.fn();
    const wsRef = { current: { send: mockSend } };
    const { useKeyboardShortcuts } = await import("../useKeyboardShortcuts");
    renderHook(() => useKeyboardShortcuts(wsRef as any));

    const approve = mockRegister.mock.calls.find((c: any) => c[0].id === "approve-current");
    approve![0].handler();
    expect(mockSend).toHaveBeenCalledWith({ type: "task_approve", data: { task_id: 42 } });
  });

  it("approve-current does nothing when no task focused", async () => {
    const mockSend = vi.fn();
    const wsRef = { current: { send: mockSend } };
    const { useKeyboardShortcuts } = await import("../useKeyboardShortcuts");
    renderHook(() => useKeyboardShortcuts(wsRef as any));

    const approve = mockRegister.mock.calls.find((c: any) => c[0].id === "approve-current");
    approve![0].handler();
    expect(mockSend).not.toHaveBeenCalled();
  });

  it("command-palette handler toggles palette open state", async () => {
    const { useKeyboardShortcuts } = await import("../useKeyboardShortcuts");
    const wsRef = { current: null };
    renderHook(() => useKeyboardShortcuts(wsRef as any));

    const palette = mockRegister.mock.calls.find((c: any) => c[0].id === "command-palette");
    palette![0].handler();
    expect(useStore.getState().isCommandPaletteOpen).toBe(true);
    palette![0].handler();
    expect(useStore.getState().isCommandPaletteOpen).toBe(false);
  });

  it("adds keydown listener on mount and removes on unmount", async () => {
    const addSpy = vi.spyOn(window, "addEventListener");
    const removeSpy = vi.spyOn(window, "removeEventListener");

    const { useKeyboardShortcuts } = await import("../useKeyboardShortcuts");
    const wsRef = { current: null };
    const { unmount } = renderHook(() => useKeyboardShortcuts(wsRef as any));

    expect(addSpy).toHaveBeenCalledWith("keydown", expect.any(Function));
    unmount();
    expect(removeSpy).toHaveBeenCalledWith("keydown", expect.any(Function));

    addSpy.mockRestore();
    removeSpy.mockRestore();
  });
});
