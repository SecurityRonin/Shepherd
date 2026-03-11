import { describe, it, expect, vi } from "vitest";
import { renderHook, act } from "@testing-library/react";
import { useDragAndDrop } from "../useDragAndDrop";

function mockDragEvent(overrides: Record<string, unknown> = {}): React.DragEvent {
  return {
    dataTransfer: {
      effectAllowed: "",
      dropEffect: "",
      setData: vi.fn(),
      getData: vi.fn(() => ""),
      ...((overrides.dataTransfer as object) ?? {}),
    },
    preventDefault: vi.fn(),
    ...overrides,
  } as unknown as React.DragEvent;
}

describe("useDragAndDrop", () => {
  it("handleDragStart sets draggedId", () => {
    const onReorder = vi.fn();
    const { result } = renderHook(() => useDragAndDrop([1, 2, 3], onReorder));

    const event = mockDragEvent();
    act(() => {
      result.current.handleDragStart(42)(event);
    });

    expect(result.current.dragState.draggedId).toBe(42);
  });

  it("handleDrop reorders items", () => {
    const onReorder = vi.fn();
    const { result } = renderHook(() => useDragAndDrop([1, 2, 3], onReorder));

    // Drag item 3 to index 0
    const dropEvent = mockDragEvent({
      dataTransfer: {
        effectAllowed: "",
        dropEffect: "",
        setData: vi.fn(),
        getData: vi.fn(() => "3"),
      },
    });

    act(() => {
      result.current.handleDrop(0)(dropEvent);
    });

    expect(onReorder).toHaveBeenCalledWith([3, 1, 2]);
  });

  it("handleDrop with same index is no-op", () => {
    const onReorder = vi.fn();
    const { result } = renderHook(() => useDragAndDrop([1, 2, 3], onReorder));

    // Drop item 2 at its current index (1)
    const dropEvent = mockDragEvent({
      dataTransfer: {
        effectAllowed: "",
        dropEffect: "",
        setData: vi.fn(),
        getData: vi.fn(() => "2"),
      },
    });

    act(() => {
      result.current.handleDrop(1)(dropEvent);
    });

    expect(onReorder).not.toHaveBeenCalled();
  });

  it("handleDragEnd resets state", () => {
    const onReorder = vi.fn();
    const { result } = renderHook(() => useDragAndDrop([1, 2, 3], onReorder));

    // First start a drag
    const startEvent = mockDragEvent();
    act(() => {
      result.current.handleDragStart(42)(startEvent);
    });
    expect(result.current.dragState.draggedId).toBe(42);

    // Then end the drag
    act(() => {
      result.current.handleDragEnd();
    });

    expect(result.current.dragState.draggedId).toBeNull();
    expect(result.current.dragState.overIndex).toBeNull();
  });
});
