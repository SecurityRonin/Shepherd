export type TaskStatus = "queued" | "running" | "input" | "review" | "error" | "done";

export interface FileDiff {
  file_path: string;
  before_content: string;
  after_content: string;
  language: string;
}

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
  iterm2_session_id?: string;
  gate_results?: { gate: string; passed: boolean }[];
  diffs?: FileDiff[];
}

export interface CreateTask {
  title: string;
  prompt?: string;
  agent_id: string;
  repo_path?: string;
  isolation_mode?: string;
}

export interface KanbanColumnDef {
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
