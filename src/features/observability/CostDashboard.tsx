import React, { useEffect } from "react";
import { useStore } from "../../store";
import { AgentSpendingRow } from "./AgentSpendingRow";
import { BudgetBar } from "./BudgetBar";
import type { BudgetAlertEvent } from "../../types/events";

const DAILY_BUDGET_USD = 5.0;

function alertSeverityColor(status: string): string {
  if (status.includes("exceeded")) return "text-red-400";
  if (status.includes("warning")) return "text-yellow-400";
  return "text-gray-400";
}

export const CostDashboard: React.FC = () => {
  const summary = useStore((s) => s.spendingSummary);
  const budgetAlerts = useStore((s) => s.budgetAlerts);
  const fetchMetrics = useStore((s) => s.fetchMetrics);

  useEffect(() => {
    fetchMetrics();
  }, [fetchMetrics]);

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

      {budgetAlerts.length > 0 && (
        <div data-testid="budget-alerts">
          <h3 className="text-sm font-medium text-gray-400 mb-2">Budget Alerts</h3>
          <div className="space-y-1">
            {budgetAlerts.map((alert: BudgetAlertEvent, i: number) => (
              <div key={i} className={`text-sm ${alertSeverityColor(alert.status)}`}>
                {alert.message}
              </div>
            ))}
          </div>
        </div>
      )}

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

      {summary.by_model.length > 0 && (
        <div>
          <h3 className="text-sm font-medium text-gray-400 mb-2">By Model</h3>
          <table className="w-full">
            <thead>
              <tr className="text-left text-xs text-gray-500">
                <th className="py-2 px-4">Model</th>
                <th className="py-2 px-4">Calls</th>
                <th className="py-2 px-4">Cost (USD)</th>
                <th className="py-2 px-4">Tokens</th>
              </tr>
            </thead>
            <tbody>
              {summary.by_model.map((m) => (
                <tr key={m.model_id}>
                  <td className="py-2 px-4 text-sm text-white">{m.model_id}</td>
                  <td className="py-2 px-4 text-sm text-gray-300">{m.call_count}</td>
                  <td className="py-2 px-4 text-sm text-yellow-400 font-mono">
                    ${m.total_cost_usd.toFixed(4)}
                  </td>
                  <td className="py-2 px-4 text-sm text-gray-300">
                    {m.total_tokens.toLocaleString()}
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      )}

      <div className="text-xs text-gray-500">
        Total: {summary.total_tasks} tasks · {summary.total_llm_calls} LLM calls ·
        ${summary.total_cost_usd.toFixed(4)}
      </div>
    </div>
  );
};
