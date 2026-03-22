import React from "react";

interface TrialBadgeProps {
  feature: string;
  remaining: number;
}

export const TrialBadge: React.FC<TrialBadgeProps> = React.memo(({ feature, remaining }) => {
  return (
    <div className="flex items-center justify-between py-1" data-testid={`trial-${feature}`}>
      <span className="text-sm text-gray-400 capitalize">{feature}</span>
      <span
        className={`text-sm font-mono ${remaining === 0 ? "text-red-400" : "text-green-400"}`}
        data-testid={`trial-count-${feature}`}
      >
        {remaining} remaining
      </span>
    </div>
  );
});
