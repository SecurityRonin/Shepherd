import React from "react";
import type { StalenessLevel } from "../../hooks/useTaskStaleness";

export interface StatusIndicatorProps {
  level: StalenessLevel;
  "data-testid"?: string;
}

const STALENESS_COLORS: Record<StalenessLevel, string> = {
  fresh: "#3fb950",
  stale: "#d29922",
  critical: "#f85149",
};

export const StatusIndicator: React.FC<StatusIndicatorProps> = ({
  level,
  "data-testid": testId,
}) => {
  const color = STALENESS_COLORS[level];

  return (
    <span
      data-testid={testId}
      className="inline-block h-2 w-2 rounded-full"
      style={{ backgroundColor: color }}
      title={`Status: ${level}`}
    />
  );
};
