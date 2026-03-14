import type { StateCreator } from "zustand";

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
  setAgentMetrics: (m: TaskMetrics[]) => void;
  setSpendingSummary: (s: SpendingSummary | null) => void;
  setReplayEvents: (e: TimelineEvent[]) => void;
}

export const createObservabilitySlice: StateCreator<ObservabilitySlice, [], [], ObservabilitySlice> = (set) => ({
  agentMetrics: [],
  spendingSummary: null,
  replayEvents: [],
  setAgentMetrics: (m) => set({ agentMetrics: m }),
  setSpendingSummary: (s) => set({ spendingSummary: s }),
  setReplayEvents: (e) => set({ replayEvents: e }),
});
