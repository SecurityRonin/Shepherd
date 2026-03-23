import { describe, it, expect, vi, beforeEach } from "vitest";
import { renderHook, act, waitFor } from "@testing-library/react";
import type { SessionState } from "../../types/task";

vi.mock("../../lib/api", () => ({
  getInterruptedSessions: vi.fn(),
  clearSessionState: vi.fn(),
  startFreshSession: vi.fn(),
}));

import { getInterruptedSessions, clearSessionState, startFreshSession } from "../../lib/api";
import { useSessionPersistence } from "../useSessionPersistence";

function makeSession(overrides: Partial<SessionState> = {}): SessionState {
  return {
    task_id: 1,
    session_id: "sess-abc",
    last_prompt: "Implement auth flow",
    working_dir: "/home/user/project",
    saved_at: "2026-03-23T10:00:00Z",
    ...overrides,
  };
}

describe("useSessionPersistence", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    vi.mocked(getInterruptedSessions).mockResolvedValue([]);
    vi.mocked(clearSessionState).mockResolvedValue({ cleared: true });
    vi.mocked(startFreshSession).mockResolvedValue({ status: "ok" });
  });

  it("fetches interrupted sessions on mount", async () => {
    const sessions = [makeSession()];
    vi.mocked(getInterruptedSessions).mockResolvedValue(sessions);

    const { result } = renderHook(() => useSessionPersistence());

    await waitFor(() => {
      expect(getInterruptedSessions).toHaveBeenCalledTimes(1);
      expect(result.current.sessions).toEqual(sessions);
    });
  });

  it("returns empty sessions when API returns empty", async () => {
    vi.mocked(getInterruptedSessions).mockResolvedValue([]);

    const { result } = renderHook(() => useSessionPersistence());

    await waitFor(() => {
      expect(result.current.sessions).toEqual([]);
    });
  });

  it("returns sessions when API returns data", async () => {
    const sessions = [
      makeSession({ task_id: 1 }),
      makeSession({ task_id: 2, last_prompt: "Fix CI" }),
    ];
    vi.mocked(getInterruptedSessions).mockResolvedValue(sessions);

    const { result } = renderHook(() => useSessionPersistence());

    await waitFor(() => {
      expect(result.current.sessions).toHaveLength(2);
      expect(result.current.sessions[0].task_id).toBe(1);
      expect(result.current.sessions[1].task_id).toBe(2);
    });
  });

  it("handles fetch error gracefully", async () => {
    vi.mocked(getInterruptedSessions).mockRejectedValue(new Error("Network error"));

    const { result } = renderHook(() => useSessionPersistence());

    await waitFor(() => {
      expect(result.current.sessions).toEqual([]);
    });
  });

  it("dismiss removes session from state and clears server state", async () => {
    const sessions = [
      makeSession({ task_id: 1 }),
      makeSession({ task_id: 2, last_prompt: "Fix CI" }),
    ];
    vi.mocked(getInterruptedSessions).mockResolvedValue(sessions);

    const { result } = renderHook(() => useSessionPersistence());

    await waitFor(() => {
      expect(result.current.sessions).toHaveLength(2);
    });

    await act(async () => {
      await result.current.dismiss(1);
    });

    expect(clearSessionState).toHaveBeenCalledWith(1);
    expect(result.current.sessions).toHaveLength(1);
    expect(result.current.sessions[0].task_id).toBe(2);
  });

  it("startFresh removes session from state and clears server state", async () => {
    const sessions = [
      makeSession({ task_id: 1 }),
      makeSession({ task_id: 2, last_prompt: "Fix CI" }),
    ];
    vi.mocked(getInterruptedSessions).mockResolvedValue(sessions);

    const { result } = renderHook(() => useSessionPersistence());

    await waitFor(() => {
      expect(result.current.sessions).toHaveLength(2);
    });

    await act(async () => {
      await result.current.startFresh(1);
    });

    expect(clearSessionState).toHaveBeenCalledWith(1);
    expect(result.current.sessions).toHaveLength(1);
    expect(result.current.sessions[0].task_id).toBe(2);
  });
});
