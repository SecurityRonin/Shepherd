import React from "react";
import { AGENT_COLORS } from "../../types/task";

export interface AgentBadgeProps {
  agentId: string;
}

export const AgentBadge: React.FC<AgentBadgeProps> = ({ agentId }) => {
  const info = AGENT_COLORS[agentId];
  const label = info?.label ?? agentId;
  const color = info?.color ?? "#6b7280";

  return (
    <span
      className="inline-flex items-center gap-1 rounded px-1.5 py-0.5 text-xs font-medium"
      style={{
        backgroundColor: `${color}20`,
        color,
        border: `1px solid ${color}40`,
      }}
    >
      {label}
    </span>
  );
};
