export interface TaskEvent {
  id: number;
  title: string;
  agent_id: string;
  status: string;
  branch: string;
  repo_path: string;
  iterm2_session_id?: string;
}

export interface PermissionEvent {
  id: number;
  task_id: number;
  tool_name: string;
  tool_args: string;
  decision: string;
}

export interface StatusSnapshot {
  tasks: TaskEvent[];
  pending_permissions: PermissionEvent[];
}

export interface MetricsUpdateEvent {
  task_id: number;
  agent_id: string;
  model_id: string;
  total_input_tokens: number;
  total_output_tokens: number;
  total_tokens: number;
  total_cost_usd: number;
  llm_calls: number;
  duration_secs: number | null;
}

export interface BudgetAlertEvent {
  scope: string;
  scope_id: string;
  status: string;
  current_cost: number;
  limit: number;
  percentage: number;
  message: string;
}

export interface GateResultEvent {
  task_id: number;
  gate: string;
  passed: boolean;
}

export type ServerEvent =
  | { type: "task_created"; data: TaskEvent }
  | { type: "task_updated"; data: TaskEvent }
  | { type: "task_deleted"; data: { id: number } }
  | { type: "terminal_output"; data: { task_id: number; data: string } }
  | { type: "permission_requested"; data: PermissionEvent }
  | { type: "permission_resolved"; data: PermissionEvent }
  | { type: "gate_result"; data: GateResultEvent }
  | { type: "notification"; data: { kind: string; title: string; body: string } }
  | { type: "status_snapshot"; data: StatusSnapshot }
  | { type: "metrics_update"; data: MetricsUpdateEvent }
  | { type: "budget_alert"; data: BudgetAlertEvent };

export type ClientEvent =
  | { type: "task_create"; data: { title: string; agent_id: string; repo_path?: string; isolation_mode?: string; prompt?: string } }
  | { type: "task_approve"; data: { task_id: number } }
  | { type: "task_approve_all"; data: null }
  | { type: "task_cancel"; data: { task_id: number } }
  | { type: "terminal_input"; data: { task_id: number; data: string } }
  | { type: "terminal_resize"; data: { task_id: number; cols: number; rows: number } }
  | { type: "subscribe"; data: null };
