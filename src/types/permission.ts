export type PermissionDecision = "auto" | "approved" | "denied" | "pending";

export interface Permission {
  id: number;
  task_id: number;
  tool_name: string;
  tool_args: string;
  decision: PermissionDecision;
  rule_matched: string | null;
  decided_at: string | null;
}
