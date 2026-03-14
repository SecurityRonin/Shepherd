import React from "react";

interface BudgetBarProps {
  used: number;
  limit: number;
  label?: string;
}

export const BudgetBar: React.FC<BudgetBarProps> = ({ used, limit, label }) => {
  const pct = limit > 0 ? Math.min((used / limit) * 100, 100) : 0;
  const isOver = pct >= 80;
  return (
    <div className="w-full">
      {label && <div className="text-xs text-gray-400 mb-1">{label}</div>}
      <div className="w-full bg-gray-700 rounded-full h-2">
        <div
          data-testid="budget-bar-fill"
          className={`h-2 rounded-full transition-all ${isOver ? "bg-red-500" : "bg-blue-500"}`}
          style={{ width: `${pct}%` }}
        />
      </div>
      <div className="text-xs text-gray-400 mt-1">${used.toFixed(4)} / ${limit.toFixed(2)}</div>
    </div>
  );
};
