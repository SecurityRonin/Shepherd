import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";
import { renderHook, act } from "@testing-library/react";
import {
  useTaskStaleness,
  FRESH_THRESHOLD_MS,
  STALE_THRESHOLD_MS,
} from "../useTaskStaleness";

describe("useTaskStaleness", () => {
  beforeEach(() => {
    vi.useFakeTimers();
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  it('returns "fresh" for recent timestamps', () => {
    const now = new Date().toISOString();
    const { result } = renderHook(() => useTaskStaleness(now, true));
    expect(result.current).toBe("fresh");
  });

  it('returns "stale" for timestamps >30s old', () => {
    const thirtyOneSecondsAgo = new Date(
      Date.now() - FRESH_THRESHOLD_MS - 1000,
    ).toISOString();
    const { result } = renderHook(() =>
      useTaskStaleness(thirtyOneSecondsAgo, true),
    );
    expect(result.current).toBe("stale");
  });

  it('returns "critical" for timestamps >2min old', () => {
    const threeMinutesAgo = new Date(
      Date.now() - STALE_THRESHOLD_MS - 1000,
    ).toISOString();
    const { result } = renderHook(() =>
      useTaskStaleness(threeMinutesAgo, true),
    );
    expect(result.current).toBe("critical");
  });

  it('returns "fresh" when isActive=false regardless of timestamp', () => {
    const tenMinutesAgo = new Date(Date.now() - 600_000).toISOString();
    const { result } = renderHook(() =>
      useTaskStaleness(tenMinutesAgo, false),
    );
    expect(result.current).toBe("fresh");
  });

  it("updates level on interval tick", () => {
    const now = new Date().toISOString();
    const { result } = renderHook(() => useTaskStaleness(now, true));
    expect(result.current).toBe("fresh");

    // Advance past FRESH_THRESHOLD_MS (30s) + one tick interval (10s)
    act(() => {
      vi.advanceTimersByTime(FRESH_THRESHOLD_MS + 10_000);
    });

    expect(result.current).toBe("stale");
  });
});
