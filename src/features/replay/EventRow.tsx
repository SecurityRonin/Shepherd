import React, { useState } from "react";
import type { TimelineEvent } from "../../store/observability";
import { EventTypeBadge } from "./EventTypeBadge";

interface EventRowProps {
  event: TimelineEvent;
}

export const EventRow: React.FC<EventRowProps> = ({ event }) => {
  const [expanded, setExpanded] = useState(false);

  return (
    <div className="border-b border-gray-700 py-2 px-4">
      <div
        className="flex items-center gap-3 cursor-pointer"
        onClick={() => setExpanded((e) => !e)}
        data-testid="event-row-header"
      >
        <EventTypeBadge type={event.event_type} />
        <span className="text-xs text-gray-400 font-mono">
          {new Date(event.timestamp).toLocaleTimeString()}
        </span>
        <span className="text-sm text-gray-200 flex-1">{event.summary}</span>
      </div>
      {expanded && event.content && (
        <pre
          className="mt-2 text-xs text-gray-300 bg-gray-800 rounded p-2 overflow-auto max-h-40"
          data-testid="event-row-content"
        >
          {event.content}
        </pre>
      )}
    </div>
  );
};
