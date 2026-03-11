import React from "react";
import type { StalenessLevel } from "../../hooks/useTaskStaleness";

export interface StatusIndicatorProps {
  level: StalenessLevel;
  size?: "sm" | "md";
  "data-testid"?: string;
}

const LEVEL_COLORS: Record<StalenessLevel, string> = {
  fresh: "bg-shepherd-green",
  stale: "bg-shepherd-yellow",
  critical: "bg-shepherd-red animate-pulse",
};

const LEVEL_LABELS: Record<StalenessLevel, string> = {
  fresh: "Active",
  stale: "Idle >30s",
  critical: "Idle >2min",
};

export const StatusIndicator: React.FC<StatusIndicatorProps> = ({
  level,
  size = "sm",
  "data-testid": testId,
}) => {
  const dotSize = size === "sm" ? "w-2 h-2" : "w-2.5 h-2.5";

  return (
    <div className="flex items-center gap-1" title={LEVEL_LABELS[level]} data-testid={testId}>
      <div className={`${dotSize} rounded-full ${LEVEL_COLORS[level]}`} />
    </div>
  );
};
