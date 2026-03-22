import { describe, it, expect, vi, beforeEach } from "vitest";
import { renderHook, act } from "@testing-library/react";

const mockConnect = vi.fn();
const mockDisconnect = vi.fn();
const mockSend = vi.fn();
let capturedOnEvent: ((event: any) => void) | null = null;
let capturedOnStatusChange: ((status: string) => void) | null = null;

vi.mock("../../lib/ws", () => ({
  createWsClient: vi.fn((opts: any) => {
    capturedOnEvent = opts.onEvent;
    capturedOnStatusChange = opts.onStatusChange;
    return {
      connect: mockConnect,
      disconnect: mockDisconnect,
      send: mockSend,
    };
  }),
}));

vi.mock("../../lib/tauri", () => ({
  getServerPort: vi.fn().mockResolvedValue(9876),
}));

beforeEach(() => {
  mockConnect.mockClear();
  mockDisconnect.mockClear();
  mockSend.mockClear();
  capturedOnEvent = null;
  capturedOnStatusChange = null;
});

describe("useWebSocket", () => {
  it("creates ws client with correct URL after resolving port", async () => {
    const { createWsClient } = await import("../../lib/ws");
    const { useWebSocket } = await import("../useWebSocket");
    const onEvent = vi.fn();
    const onStatus = vi.fn();

    renderHook(() => useWebSocket(onEvent, onStatus));

    // Wait for port resolution
    await act(async () => {});

    expect(createWsClient).toHaveBeenCalledWith(
      expect.objectContaining({ url: "ws://127.0.0.1:9876/ws" }),
    );
  });

  it("connects and sends subscribe message on mount", async () => {
    const { useWebSocket } = await import("../useWebSocket");
    renderHook(() => useWebSocket(vi.fn(), vi.fn()));

    await act(async () => {});

    expect(mockConnect).toHaveBeenCalled();
    expect(mockSend).toHaveBeenCalledWith({ type: "subscribe", data: null });
  });

  it("disconnects on unmount", async () => {
    const { useWebSocket } = await import("../useWebSocket");
    const { unmount } = renderHook(() => useWebSocket(vi.fn(), vi.fn()));

    await act(async () => {});

    unmount();
    expect(mockDisconnect).toHaveBeenCalled();
  });

  it("forwards server events to onEvent callback", async () => {
    const { useWebSocket } = await import("../useWebSocket");
    const onEvent = vi.fn();
    renderHook(() => useWebSocket(onEvent, vi.fn()));

    await act(async () => {});

    const fakeEvent = { type: "task_created", data: { id: 1 } };
    capturedOnEvent?.(fakeEvent);
    expect(onEvent).toHaveBeenCalledWith(fakeEvent);
  });

  it("forwards status changes to onStatusChange callback", async () => {
    const { useWebSocket } = await import("../useWebSocket");
    const onStatus = vi.fn();
    renderHook(() => useWebSocket(vi.fn(), onStatus));

    await act(async () => {});

    capturedOnStatusChange?.("connected");
    expect(onStatus).toHaveBeenCalledWith("connected");
  });

  it("returns a ref containing the ws client", async () => {
    const { useWebSocket } = await import("../useWebSocket");
    const { result } = renderHook(() => useWebSocket(vi.fn(), vi.fn()));

    await act(async () => {});

    expect(result.current.current).toBeDefined();
    expect(result.current.current?.connect).toBe(mockConnect);
  });
});
