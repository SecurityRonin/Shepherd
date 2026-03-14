import React, { useEffect } from "react";
import { useStore } from "../../store";
import { EventRow } from "./EventRow";

interface ReplayViewerProps {
  taskId?: number;
}

export const ReplayViewer: React.FC<ReplayViewerProps> = ({ taskId }) => {
  const events = useStore((s) => s.replayEvents);
  const setReplayEvents = useStore((s) => s.setReplayEvents);

  useEffect(() => {
    if (!taskId) return;
    fetch(`/api/replay/task/${taskId}`)
      .then((r) => r.json())
      .then(setReplayEvents)
      .catch(() => {});
  }, [taskId, setReplayEvents]);

  if (events.length === 0) {
    return (
      <div className="p-6 text-center text-gray-400" data-testid="no-events">
        No events
      </div>
    );
  }

  return (
    <div className="flex flex-col h-full overflow-auto">
      <h2 className="text-xl font-semibold text-white p-4">Replay</h2>
      <div className="flex-1 overflow-auto">
        {events.map((e) => (
          <EventRow key={e.id} event={e} />
        ))}
      </div>
    </div>
  );
};
