import React, { useEffect } from "react";
import { useStore } from "../../store";
import { EventRow } from "./EventRow";
import { getReplayEvents } from "../../lib/api";

interface ReplayViewerProps {
  taskId?: number;
}

export const ReplayViewer: React.FC<ReplayViewerProps> = ({ taskId }) => {
  const events = useStore((s) => s.replayEvents);
  const setReplayEvents = useStore((s) => s.setReplayEvents);
  const [error, setError] = React.useState<string | null>(null);

  useEffect(() => {
    if (!taskId) return;
    getReplayEvents(taskId)
      .then((data) => { setError(null); setReplayEvents(data); })
      .catch((err) => setError(err instanceof Error ? err.message : "Failed to load events"));
  }, [taskId, setReplayEvents]);

  if (error) {
    return (
      <div className="p-6 text-center text-red-400" data-testid="error-message">
        {error}
      </div>
    );
  }

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
