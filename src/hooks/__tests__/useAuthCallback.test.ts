import { describe, it, expect, vi, beforeEach } from "vitest";
import { renderHook, act } from "@testing-library/react";

// Track listen registrations
const listeners = new Map<string, (event: { payload: unknown }) => void>();
const mockUnlisten = vi.fn();

vi.mock("../../lib/tauri", () => ({
  listen: vi.fn(async (event: string, handler: (event: { payload: unknown }) => void) => {
    listeners.set(event, handler);
    return mockUnlisten;
  }),
}));

// Mock api module — getCloudStatus is called on auth success to refresh state
vi.mock("../../lib/api", () => ({
  getCloudStatus: vi.fn().mockResolvedValue({
    cloud_available: true,
    authenticated: true,
    plan: "pro",
    credits_balance: 100,
    cloud_generation_enabled: true,
  }),
}));

beforeEach(() => {
  listeners.clear();
  mockUnlisten.mockClear();
  vi.clearAllMocks();
});

describe("useAuthCallback", () => {
  it("registers listeners for auth-callback-success and auth-callback-error", async () => {
    const { listen } = await import("../../lib/tauri");
    const { useAuthCallback } = await import("../useAuthCallback");
    renderHook(() => useAuthCallback());

    // Should have registered two listeners
    expect(listen).toHaveBeenCalledWith("auth-callback-success", expect.any(Function));
    expect(listen).toHaveBeenCalledWith("auth-callback-error", expect.any(Function));
  });

  it("calls onSuccess callback when auth-callback-success fires", async () => {
    const { useAuthCallback } = await import("../useAuthCallback");
    const onSuccess = vi.fn();
    renderHook(() => useAuthCallback({ onSuccess }));

    // Simulate Tauri emitting the success event
    const handler = listeners.get("auth-callback-success");
    expect(handler).toBeDefined();
    await act(async () => {
      handler!({ payload: { user_id: "u1", email: "a@b.com" } });
    });

    expect(onSuccess).toHaveBeenCalledWith({ user_id: "u1", email: "a@b.com" });
  });

  it("calls onError callback when auth-callback-error fires", async () => {
    const { useAuthCallback } = await import("../useAuthCallback");
    const onError = vi.fn();
    renderHook(() => useAuthCallback({ onError }));

    const handler = listeners.get("auth-callback-error");
    expect(handler).toBeDefined();
    await act(async () => {
      handler!({ payload: { error: "token_expired" } });
    });

    expect(onError).toHaveBeenCalledWith({ error: "token_expired" });
  });

  it("unlistens on unmount", async () => {
    const { useAuthCallback } = await import("../useAuthCallback");
    const { unmount } = renderHook(() => useAuthCallback());

    // Wait for the listen promises to resolve
    await act(async () => {});

    unmount();

    // The cleanup runs the unlisten promises asynchronously; flush microtasks
    await act(async () => {});

    // Two listeners registered, so unlisten should be called twice
    expect(mockUnlisten).toHaveBeenCalledTimes(2);
  });
});
