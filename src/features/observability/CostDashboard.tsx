import React, { useEffect } from "react";
import { useStore } from "../../store";
import { AgentSpendingRow } from "./AgentSpendingRow";
import { BudgetBar } from "./BudgetBar";

const DAILY_BUDGET_USD = 5.0;

export const CostDashboard: React.FC = () => {
  const summary = useStore((s) => s.spendingSummary);
  const setSpendingSummary = useStore((s) => s.setSpendingSummary);

  useEffect(() => {
    fetch("/api/observability/summary")
      .then((r) => r.json())
      .then(setSpendingSummary)
      .catch(() => {});
  }, [setSpendingSummary]);

  if (!summary) {
    return (
      <div className="p-6 text-center text-gray-400" data-testid="no-spending">
        No spending data
      </div>
    );
  }

  return (
    <div className="p-6 space-y-6">
      <h2 className="text-xl font-semibold text-white">Cost Dashboard</h2>
      <BudgetBar
        used={summary.total_cost_usd}
        limit={DAILY_BUDGET_USD}
        label="Daily Budget"
      />
      <div>
        <h3 className="text-sm font-medium text-gray-400 mb-2">Agent Spending</h3>
        {summary.by_agent.length === 0 ? (
          <p className="text-gray-500 text-sm">No spending data</p>
        ) : (
          <table className="w-full">
            <thead>
              <tr className="text-left text-xs text-gray-500">
                <th className="py-2 px-4">Agent</th>
                <th className="py-2 px-4">Tasks</th>
                <th className="py-2 px-4">Cost (USD)</th>
                <th className="py-2 px-4">Tokens</th>
              </tr>
            </thead>
            <tbody>
              {summary.by_agent.map((a) => (
                <AgentSpendingRow key={a.agent_id} spending={a} />
              ))}
            </tbody>
          </table>
        )}
      </div>
      <div className="text-xs text-gray-500">
        Total: {summary.total_tasks} tasks · {summary.total_llm_calls} LLM calls
      </div>
    </div>
  );
};
