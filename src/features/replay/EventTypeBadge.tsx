import React from "react";

const BADGE_COLORS: Record<string, string> = {
  session_start: "bg-green-600 text-white",
  session_end: "bg-gray-600 text-white",
  tool_call: "bg-blue-600 text-white",
  tool_result: "bg-blue-400 text-white",
  output: "bg-gray-500 text-white",
  error: "bg-red-600 text-white",
  permission_request: "bg-yellow-600 text-white",
  permission_resolve: "bg-yellow-400 text-white",
  file_change: "bg-purple-600 text-white",
  llm_call: "bg-indigo-600 text-white",
  input: "bg-teal-600 text-white",
};

interface EventTypeBadgeProps {
  type: string;
}

export const EventTypeBadge: React.FC<EventTypeBadgeProps> = ({ type }) => {
  const color = BADGE_COLORS[type] ?? "bg-gray-600 text-white";
  return (
    <span
      data-testid={`badge-${type}`}
      className={`px-2 py-0.5 rounded text-xs font-mono ${color}`}
    >
      {type}
    </span>
  );
};
