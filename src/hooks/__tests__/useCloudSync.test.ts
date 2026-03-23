import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";
import { renderHook, act } from "@testing-library/react";
import { useStore } from "../../store";
import type { Task } from "../../types/task";

// Mock the api module
vi.mock("../../lib/api", () => ({
  getCloudStatus: vi.fn(),
  syncTasksToCloud: vi.fn(),
}));

import { getCloudStatus, syncTasksToCloud } from "../../lib/api";
import { useCloudSync } from "../useCloudSync";

function makeTask(overrides: Partial<Task> = {}): Task {
  return {
    id: 1,
    title: "Test task",
    prompt: "Do something",
    agent_id: "claude-code",
    repo_path: "/tmp/repo",
    branch: "main",
    isolation_mode: "worktree",
    status: "running",
    created_at: "2026-01-01T00:00:00Z",
    updated_at: "2026-01-01T00:00:00Z",
    ...overrides,
  };
}

describe("useCloudSync", () => {
  let hookResult: ReturnType<typeof renderHook> | null = null;

  beforeEach(() => {
    vi.useFakeTimers();
    vi.clearAllMocks();
    // Reset store to clean state
    useStore.setState({ tasks: {} });
    // Default: cloud available and authenticated
    vi.mocked(getCloudStatus).mockResolvedValue({
      cloud_available: true,
      authenticated: true,
      plan: "pro",
      credits_balance: 100,
      cloud_generation_enabled: true,
    });
    vi.mocked(syncTasksToCloud).mockResolvedValue({ synced: 1 });
    hookResult = null;
  });

  afterEach(() => {
    // Ensure hook is unmounted to clean up subscriptions
    if (hookResult) {
      hookResult.unmount();
      hookResult = null;
    }
    vi.useRealTimers();
  });

  it("calls getCloudStatus on mount", async () => {
    hookResult = renderHook(() => useCloudSync());

    await act(async () => {
      await vi.runAllTimersAsync();
    });

    expect(getCloudStatus).toHaveBeenCalledTimes(1);
  });

  it("does not sync when cloud is not available", async () => {
    vi.mocked(getCloudStatus).mockResolvedValue({
      cloud_available: false,
      authenticated: true,
      plan: "pro",
      credits_balance: 100,
      cloud_generation_enabled: true,
    });

    hookResult = renderHook(() => useCloudSync());

    await act(async () => {
      await vi.runAllTimersAsync();
    });

    // Change tasks -- should NOT trigger sync
    act(() => {
      useStore.setState({ tasks: { 1: makeTask() } });
    });

    await act(async () => {
      await vi.advanceTimersByTimeAsync(6000);
    });

    expect(syncTasksToCloud).not.toHaveBeenCalled();
  });

  it("does not sync when not authenticated", async () => {
    vi.mocked(getCloudStatus).mockResolvedValue({
      cloud_available: true,
      authenticated: false,
      plan: null,
      credits_balance: null,
      cloud_generation_enabled: false,
    });

    hookResult = renderHook(() => useCloudSync());

    await act(async () => {
      await vi.runAllTimersAsync();
    });

    // Change tasks -- should NOT trigger sync
    act(() => {
      useStore.setState({ tasks: { 1: makeTask() } });
    });

    await act(async () => {
      await vi.advanceTimersByTimeAsync(6000);
    });

    expect(syncTasksToCloud).not.toHaveBeenCalled();
  });

  it("subscribes to store changes and pushes when authenticated", async () => {
    hookResult = renderHook(() => useCloudSync());

    // Wait for mount + auth check
    await act(async () => {
      await vi.runAllTimersAsync();
    });

    // Trigger a task change
    act(() => {
      useStore.setState({ tasks: { 1: makeTask() } });
    });

    // Advance past the 5-second debounce
    await act(async () => {
      await vi.advanceTimersByTimeAsync(5000);
    });

    expect(syncTasksToCloud).toHaveBeenCalledTimes(1);
    expect(syncTasksToCloud).toHaveBeenCalledWith([makeTask()]);
  });

  it("debounces pushes with a 5-second timeout", async () => {
    hookResult = renderHook(() => useCloudSync());

    await act(async () => {
      await vi.runAllTimersAsync();
    });

    // Trigger multiple rapid task changes
    act(() => {
      useStore.setState({ tasks: { 1: makeTask({ title: "First" }) } });
    });

    // Advance 3 seconds (not yet at 5s debounce)
    await act(async () => {
      await vi.advanceTimersByTimeAsync(3000);
    });

    // Should not have synced yet
    expect(syncTasksToCloud).not.toHaveBeenCalled();

    // Trigger another change -- resets the debounce timer
    act(() => {
      useStore.setState({ tasks: { 1: makeTask({ title: "Second" }) } });
    });

    // Advance 4 seconds (total 7s from first change, but only 4s from second)
    await act(async () => {
      await vi.advanceTimersByTimeAsync(4000);
    });

    // Still should not have synced (debounce reset to 5s from second change)
    expect(syncTasksToCloud).not.toHaveBeenCalled();

    // Advance remaining 1 second to hit 5s from the second change
    await act(async () => {
      await vi.advanceTimersByTimeAsync(1000);
    });

    // Now it should sync with the latest state
    expect(syncTasksToCloud).toHaveBeenCalledTimes(1);
    expect(syncTasksToCloud).toHaveBeenCalledWith([makeTask({ title: "Second" })]);
  });

  it("does not push when tasks are empty", async () => {
    hookResult = renderHook(() => useCloudSync());

    await act(async () => {
      await vi.runAllTimersAsync();
    });

    // Set tasks to a non-empty state first, then back to empty
    act(() => {
      useStore.setState({ tasks: { 1: makeTask() } });
    });
    act(() => {
      useStore.setState({ tasks: {} });
    });

    await act(async () => {
      await vi.advanceTimersByTimeAsync(5000);
    });

    expect(syncTasksToCloud).not.toHaveBeenCalled();
  });

  it("cleans up subscription and timer on unmount", async () => {
    hookResult = renderHook(() => useCloudSync());

    await act(async () => {
      await vi.runAllTimersAsync();
    });

    // Trigger a change so a debounce timer is pending
    act(() => {
      useStore.setState({ tasks: { 1: makeTask() } });
    });

    // Unmount before the debounce fires
    hookResult.unmount();
    hookResult = null; // prevent afterEach from double-unmounting

    // Advance past debounce -- should NOT push since unmounted
    await act(async () => {
      await vi.advanceTimersByTimeAsync(6000);
    });

    expect(syncTasksToCloud).not.toHaveBeenCalled();
  });

  it("handles pull errors gracefully without crashing", async () => {
    const consoleSpy = vi.spyOn(console, "error").mockImplementation(() => {});
    vi.mocked(getCloudStatus).mockRejectedValue(new Error("Network error"));

    // Should not throw
    expect(() => {
      hookResult = renderHook(() => useCloudSync());
    }).not.toThrow();

    await act(async () => {
      await vi.runAllTimersAsync();
    });

    // After a failed pull, tasks changes should not trigger sync
    act(() => {
      useStore.setState({ tasks: { 1: makeTask() } });
    });

    await act(async () => {
      await vi.advanceTimersByTimeAsync(6000);
    });

    expect(syncTasksToCloud).not.toHaveBeenCalled();
    consoleSpy.mockRestore();
  });

  it("handles push errors gracefully without crashing", async () => {
    const consoleSpy = vi.spyOn(console, "error").mockImplementation(() => {});
    vi.mocked(syncTasksToCloud).mockRejectedValue(new Error("Push failed"));

    hookResult = renderHook(() => useCloudSync());

    await act(async () => {
      await vi.runAllTimersAsync();
    });

    // Trigger a task change
    act(() => {
      useStore.setState({ tasks: { 1: makeTask() } });
    });

    // Should not throw when push fails
    await act(async () => {
      await vi.advanceTimersByTimeAsync(5000);
    });

    expect(syncTasksToCloud).toHaveBeenCalledTimes(1);
    expect(consoleSpy).toHaveBeenCalledWith(
      "[cloud-sync] Push failed:",
      expect.any(Error),
    );
    consoleSpy.mockRestore();
  });

  it("does not push concurrently when already syncing", async () => {
    // Make syncTasksToCloud take a while to resolve
    let resolvePush!: () => void;
    vi.mocked(syncTasksToCloud).mockImplementation(
      () =>
        new Promise((resolve) => {
          resolvePush = () => resolve({ synced: 1 });
        }),
    );

    hookResult = renderHook(() => useCloudSync());

    await act(async () => {
      await vi.runAllTimersAsync();
    });

    // First change
    act(() => {
      useStore.setState({ tasks: { 1: makeTask({ title: "First" }) } });
    });

    // Fire first push
    await act(async () => {
      await vi.advanceTimersByTimeAsync(5000);
    });

    expect(syncTasksToCloud).toHaveBeenCalledTimes(1);

    // Second change while first push is still in-flight
    act(() => {
      useStore.setState({ tasks: { 1: makeTask({ title: "Second" }) } });
    });

    // Advance debounce -- pushToCloud should be called but bail out due to isSyncing
    await act(async () => {
      await vi.advanceTimersByTimeAsync(5000);
    });

    // syncTasksToCloud should NOT be called again because isSyncing is true
    expect(syncTasksToCloud).toHaveBeenCalledTimes(1);

    // Resolve the first push
    await act(async () => {
      resolvePush();
    });

    // Now a third change should work
    act(() => {
      useStore.setState({ tasks: { 1: makeTask({ title: "Third" }) } });
    });

    await act(async () => {
      await vi.advanceTimersByTimeAsync(5000);
    });

    expect(syncTasksToCloud).toHaveBeenCalledTimes(2);
  });
});
