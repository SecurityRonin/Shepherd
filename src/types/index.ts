export type { Task, TaskStatus, CreateTask, KanbanColumnDef, AgentInfo } from "./task";
export { AGENT_COLORS } from "./task";
export type { Session } from "./session";
export type { Permission, PermissionDecision } from "./permission";
export type { TaskEvent, PermissionEvent, StatusSnapshot, ServerEvent, ClientEvent, MetricsUpdateEvent, BudgetAlertEvent, GateResultEvent } from "./events";

// ── Templates ────────────────────────────────────────────────────

export type TemplateCategory = "workflow" | "pipeline" | "pair";

export interface AgentRole {
  role: string;
  agent_type: string;
  config: Record<string, unknown>;
}

export interface AgentTemplate {
  id: string;
  name: string;
  description: string;
  category: TemplateCategory;
  agents: AgentRole[];
  quality_gates: string[];
  is_premium: boolean;
}
