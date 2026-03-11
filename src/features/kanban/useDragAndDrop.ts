import { useState, useCallback } from "react";

export interface DragState {
  draggedId: number | null;
  overIndex: number | null;
}

export interface UseDragAndDropReturn {
  dragState: DragState;
  handleDragStart: (id: number) => (e: React.DragEvent) => void;
  handleDragOver: (index: number) => (e: React.DragEvent) => void;
  handleDragEnd: () => void;
  handleDrop: (index: number) => (e: React.DragEvent) => void;
}

export function useDragAndDrop(
  items: number[],
  onReorder: (newOrder: number[]) => void,
): UseDragAndDropReturn {
  const [dragState, setDragState] = useState<DragState>({
    draggedId: null,
    overIndex: null,
  });

  const handleDragStart = useCallback(
    (id: number) => (e: React.DragEvent) => {
      e.dataTransfer.effectAllowed = "move";
      e.dataTransfer.setData("text/plain", String(id));
      setDragState({ draggedId: id, overIndex: null });
    },
    [],
  );

  const handleDragOver = useCallback(
    (index: number) => (e: React.DragEvent) => {
      e.preventDefault();
      e.dataTransfer.dropEffect = "move";
      setDragState((prev) => ({ ...prev, overIndex: index }));
    },
    [],
  );

  const handleDragEnd = useCallback(() => {
    setDragState({ draggedId: null, overIndex: null });
  }, []);

  const handleDrop = useCallback(
    (targetIndex: number) => (e: React.DragEvent) => {
      e.preventDefault();
      const draggedIdStr = e.dataTransfer.getData("text/plain");
      const draggedId = parseInt(draggedIdStr, 10);
      if (isNaN(draggedId)) return;
      const currentIndex = items.indexOf(draggedId);
      if (currentIndex === -1 || currentIndex === targetIndex) {
        setDragState({ draggedId: null, overIndex: null });
        return;
      }
      const newOrder = [...items];
      newOrder.splice(currentIndex, 1);
      newOrder.splice(targetIndex, 0, draggedId);
      onReorder(newOrder);
      setDragState({ draggedId: null, overIndex: null });
    },
    [items, onReorder],
  );

  return { dragState, handleDragStart, handleDragOver, handleDragEnd, handleDrop };
}
