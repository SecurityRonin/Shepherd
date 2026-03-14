import React from "react";
import type { AgentSpending } from "../../store/observability";

interface AgentSpendingRowProps {
  spending: AgentSpending;
}

export const AgentSpendingRow: React.FC<AgentSpendingRowProps> = ({ spending }) => {
  return (
    <tr>
      <td className="py-2 px-4 text-sm text-white">{spending.agent_id}</td>
      <td className="py-2 px-4 text-sm text-gray-300">{spending.task_count}</td>
      <td className="py-2 px-4 text-sm text-yellow-400 font-mono">
        ${spending.total_cost_usd.toFixed(4)}
      </td>
      <td className="py-2 px-4 text-sm text-gray-300">
        {spending.total_tokens.toLocaleString()}
      </td>
    </tr>
  );
};
