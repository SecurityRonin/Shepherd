import type { StateCreator } from "zustand";
import type { MetricsUpdateEvent, BudgetAlertEvent, GateResultEvent } from "../types/events";

export interface TaskMetrics {
  task_id: number;
  agent_id: string;
  model_id: string;
  total_input_tokens: number;
  total_output_tokens: number;
  total_tokens: number;
  total_cost_usd: number;
  llm_calls: number;
  duration_secs: number | null;
  status: string;
  created_at: string;
  updated_at: string;
}

export interface AgentSpending {
  agent_id: string;
  total_cost_usd: number;
  total_tokens: number;
  task_count: number;
}

export interface ModelSpending {
  model_id: string;
  total_cost_usd: number;
  total_tokens: number;
  call_count: number;
}

export interface SpendingSummary {
  total_cost_usd: number;
  total_tokens: number;
  total_tasks: number;
  total_llm_calls: number;
  by_agent: AgentSpending[];
  by_model: ModelSpending[];
}

export interface TimelineEvent {
  id: number;
  task_id: number;
  session_id: number;
  event_type: string;
  summary: string;
  content: string;
  metadata: string | null;
  timestamp: string;
}

export interface ObservabilitySlice {
  agentMetrics: TaskMetrics[];
  spendingSummary: SpendingSummary | null;
  replayEvents: TimelineEvent[];
  gateResults: Record<number, GateResultEvent[]>;
  budgetAlerts: BudgetAlertEvent[];
  setAgentMetrics: (m: TaskMetrics[]) => void;
  setSpendingSummary: (s: SpendingSummary | null) => void;
  setReplayEvents: (e: TimelineEvent[]) => void;
  handleMetricsUpdate: (event: MetricsUpdateEvent) => void;
  handleGateResult: (event: GateResultEvent) => void;
  handleBudgetAlert: (event: BudgetAlertEvent) => void;
  fetchMetrics: () => Promise<void>;
}

export const createObservabilitySlice: StateCreator<ObservabilitySlice, [], [], ObservabilitySlice> = (set) => ({
  agentMetrics: [],
  spendingSummary: null,
  replayEvents: [],
  gateResults: {},
  budgetAlerts: [],
  setAgentMetrics: (m) => set({ agentMetrics: m }),
  setSpendingSummary: (s) => set({ spendingSummary: s }),
  setReplayEvents: (e) => set({ replayEvents: e }),

  handleMetricsUpdate: (event) => {
    set((state) => {
      const existing = state.agentMetrics.filter((m) => m.task_id !== event.task_id);
      const updated: TaskMetrics = {
        ...event,
        status: "running",
        created_at: new Date().toISOString(),
        updated_at: new Date().toISOString(),
      };
      return { agentMetrics: [...existing, updated] };
    });
  },

  handleGateResult: (event) => {
    set((state) => {
      const taskGates = state.gateResults[event.task_id] ?? [];
      return {
        gateResults: {
          ...state.gateResults,
          [event.task_id]: [...taskGates, event],
        },
      };
    });
  },

  handleBudgetAlert: (event) => {
    set((state) => ({
      budgetAlerts: [...state.budgetAlerts, event],
    }));
  },

  fetchMetrics: async () => {
    try {
      const { getSpendingSummary } = await import("../lib/api");
      const summary = await getSpendingSummary();
      set({ spendingSummary: summary });
    } catch {
      // Silently fail — dashboard will show empty state
    }
  },
});
