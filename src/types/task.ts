export type TaskStatus = "queued" | "running" | "input" | "review" | "error" | "done";

export interface Task {
  id: number;
  title: string;
  prompt: string;
  agent_id: string;
  repo_path: string;
  branch: string;
  isolation_mode: string;
  status: TaskStatus;
  created_at: string;
  updated_at: string;
}

export interface CreateTask {
  title: string;
  prompt?: string;
  agent_id: string;
  repo_path?: string;
  isolation_mode?: string;
}

export interface KanbanColumn {
  id: TaskStatus;
  label: string;
  tasks: Task[];
}

export interface AgentInfo {
  id: string;
  label: string;
  color: string;
}

export const AGENT_COLORS: Record<string, AgentInfo> = {
  "claude-code": { id: "claude-code", label: "Claude", color: "#d97706" },
  "codex-cli": { id: "codex-cli", label: "Codex", color: "#059669" },
  "opencode": { id: "opencode", label: "OpenCode", color: "#7c3aed" },
  "gemini-cli": { id: "gemini-cli", label: "Gemini", color: "#2563eb" },
  "aider": { id: "aider", label: "Aider", color: "#dc2626" },
};
