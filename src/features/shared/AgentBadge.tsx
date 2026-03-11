import React from "react";
import { AGENT_COLORS } from "../../types/task";

export interface AgentBadgeProps {
  agentId: string;
  size?: "sm" | "md";
}

export const AgentBadge: React.FC<AgentBadgeProps> = ({ agentId, size = "sm" }) => {
  const info = AGENT_COLORS[agentId];
  const label = info?.label ?? agentId;
  const color = info?.color ?? "#6b7280";
  const sizeClasses = size === "sm" ? "text-[10px] px-1.5 py-0.5" : "text-xs px-2 py-0.5";

  return (
    <span
      className={`inline-flex items-center rounded font-medium ${sizeClasses}`}
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
