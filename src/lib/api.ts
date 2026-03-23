import type { Task, CreateTask, SessionState } from "../types/task";
import { getServerPort } from "./tauri";

async function getBaseUrl(): Promise<string> {
  const port = await getServerPort();
  return `http://127.0.0.1:${port}`;
}

export class ApiError extends Error {
  constructor(
    public status: number,
    public body: unknown,
  ) {
    super(`API error ${status}: ${JSON.stringify(body)}`);
    this.name = "ApiError";
  }
}

async function request<T>(path: string, options?: RequestInit): Promise<T> {
  const url = `${await getBaseUrl()}${path}`;
  const response = await fetch(url, {
    ...options,
    headers: { "Content-Type": "application/json", ...options?.headers },
  });
  if (!response.ok) {
    let body: unknown;
    try {
      body = await response.json();
    } catch {
      body = await response.text();
    }
    throw new ApiError(response.status, body);
  }
  if (response.status === 204) return undefined as T;
  return response.json() as Promise<T>;
}

export interface HealthResponse {
  status: string;
  version: string;
}

export async function checkHealth(): Promise<HealthResponse> {
  return request<HealthResponse>("/api/health");
}

export async function listTasks(): Promise<Task[]> {
  return request<Task[]>("/api/tasks");
}

export async function getTask(id: number): Promise<Task> {
  return request<Task>(`/api/tasks/${id}`);
}

export async function createTask(task: CreateTask): Promise<Task> {
  return request<Task>("/api/tasks", {
    method: "POST",
    body: JSON.stringify(task),
  });
}

export async function deleteTask(id: number): Promise<{ deleted: number }> {
  return request<{ deleted: number }>(`/api/tasks/${id}`, {
    method: "DELETE",
  });
}

export async function approveTask(id: number): Promise<{ status: string }> {
  return request<{ status: string }>(`/api/tasks/${id}/approve`, {
    method: "POST",
  });
}

export async function cancelTask(id: number): Promise<{ status: string }> {
  return request<{ status: string }>(`/api/tasks/${id}/cancel`, {
    method: "POST",
  });
}

// ── Task Summaries ──────────────────────────────────────────────

export interface TaskSummaryResponse {
  summary: string;
  generated_at: string;
}

export async function getTaskSummary(taskId: number): Promise<TaskSummaryResponse> {
  return request<TaskSummaryResponse>(`/api/tasks/${taskId}/summary`);
}

// ── Cloud / Auth ─────────────────────────────────────────────────

export interface CloudStatusResponse {
  cloud_available: boolean;
  authenticated: boolean;
  plan: string | null;
  credits_balance: number | null;
  cloud_generation_enabled: boolean;
}

export interface LoginResponse {
  login_url: string;
}

export interface ProfileResponse {
  user_id: string;
  email: string | null;
  display_name: string | null;
  plan: string;
  credits_balance: number;
}

export interface CreditBalanceResponse {
  plan: string;
  credits_balance: number;
  subscription_url: string;
  topup_url: string;
}

export async function getCloudStatus(): Promise<CloudStatusResponse> {
  return request<CloudStatusResponse>("/api/cloud/status");
}

export async function getLoginUrl(): Promise<LoginResponse> {
  return request<LoginResponse>("/api/auth/login", { method: "POST" });
}

export async function getProfile(): Promise<ProfileResponse> {
  return request<ProfileResponse>("/api/auth/profile");
}

export async function logout(): Promise<{ success: boolean }> {
  return request<{ success: boolean }>("/api/auth/logout", { method: "POST" });
}

export async function getBalance(): Promise<CreditBalanceResponse> {
  return request<CreditBalanceResponse>("/api/cloud/balance");
}

// ── Cloud Sync ──────────────────────────────────────────────────

export async function syncTasksToCloud(tasks: Task[]): Promise<{ synced: number }> {
  return request<{ synced: number }>("/api/cloud/sync", {
    method: "POST",
    body: JSON.stringify({ tasks }),
  });
}

// ── Sessions (iTerm2 / Claude Code) ─────────────────────────────

export interface ClaudeSessionsResponse {
  sessions: string[];
}

export async function getClaudeSessions(taskId: number): Promise<ClaudeSessionsResponse> {
  return request<ClaudeSessionsResponse>(`/api/sessions/${taskId}/claude-sessions`);
}

export async function resumeClaudeSession(taskId: number, claudeSessionId: string): Promise<{ status: string }> {
  return request<{ status: string }>(`/api/sessions/${taskId}/resume`, {
    method: "POST",
    body: JSON.stringify({ claude_session_id: claudeSessionId }),
  });
}

export async function startFreshSession(taskId: number): Promise<{ status: string }> {
  return request<{ status: string }>(`/api/sessions/${taskId}/fresh`, {
    method: "POST",
  });
}

// ── Session Persistence ──────────────────────────────────────────

export async function getInterruptedSessions(): Promise<SessionState[]> {
  return request<SessionState[]>("/api/sessions/interrupted");
}

export async function saveSessionState(state: SessionState): Promise<{ saved: boolean }> {
  return request<{ saved: boolean }>("/api/sessions/state", {
    method: "POST",
    body: JSON.stringify(state),
  });
}

export async function clearSessionState(taskId: number): Promise<{ cleared: boolean }> {
  return request<{ cleared: boolean }>(`/api/sessions/${taskId}/state`, {
    method: "DELETE",
  });
}

// ── Templates ────────────────────────────────────────────────────

export interface TemplatesResponse {
  templates: import("../types").AgentTemplate[];
}

export async function getTemplates(
  category?: string,
  includePremium?: boolean,
): Promise<TemplatesResponse> {
  const params = new URLSearchParams();
  if (category) params.set("category", category);
  if (includePremium !== undefined) params.set("include_premium", String(includePremium));
  const query = params.toString();
  return request<TemplatesResponse>(`/api/templates${query ? `?${query}` : ""}`);
}

// ── Plugins ─────────────────────────────────────────────────────

export interface DetectedPluginsResponse {
  detected: string[];
}

export async function getDetectedPlugins(): Promise<DetectedPluginsResponse> {
  return request<DetectedPluginsResponse>("/api/plugins/detected");
}

// ── Replay ──────────────────────────────────────────────────────

export async function getReplayEvents(taskId: number): Promise<import("../store/observability").TimelineEvent[]> {
  return request(`/api/replay/task/${taskId}`);
}

// ── Triggers ─────────────────────────────────────────────────────

export interface TriggerSuggestion {
  id: string;
  tool: string;
  message: string;
  action_label: string;
  action_route: string;
  priority: "low" | "medium" | "high";
}

export async function checkTriggers(projectDir: string): Promise<TriggerSuggestion[]> {
  return request<TriggerSuggestion[]>("/api/triggers/check", {
    method: "POST",
    body: JSON.stringify({ project_dir: projectDir }),
  });
}

export async function dismissTrigger(triggerId: string, projectDir: string): Promise<{ success: boolean }> {
  return request<{ success: boolean }>("/api/triggers/dismiss", {
    method: "POST",
    body: JSON.stringify({ trigger_id: triggerId, project_dir: projectDir }),
  });
}

// ── PR Pipeline ──────────────────────────────────────────────────

export interface CreatePrRequest {
  base_branch: string;
  auto_commit_message: boolean;
  run_gates: boolean;
}

export interface PrStepResult {
  name: string;
  status: string;
  output: string;
}

export interface CreatePrResponse {
  success: boolean;
  pr_url: string | null;
  steps: PrStepResult[];
}

export async function createPr(taskId: number, params: CreatePrRequest): Promise<CreatePrResponse> {
  return request<CreatePrResponse>(`/api/tasks/${taskId}/pr`, {
    method: "POST",
    body: JSON.stringify(params),
  });
}

// ── Logo Generation ──────────────────────────────────────────────

export interface GenerateLogoRequest {
  product_name: string;
  product_description?: string;
  style: string;
  colors: string[];
}

export interface LogoVariant {
  index: number;
  image_data: string;
  is_url: boolean;
}

export interface GenerateLogoResponse {
  variants: LogoVariant[];
}

export async function generateLogo(params: GenerateLogoRequest): Promise<GenerateLogoResponse> {
  return request<GenerateLogoResponse>("/api/logogen", {
    method: "POST",
    body: JSON.stringify(params),
  });
}

export interface ExportLogoRequest {
  image_base64: string;
  product_name: string;
}

export interface ExportedFile {
  path: string;
  format: string;
  size_bytes: number;
  dimensions: [number, number] | null;
}

export interface ExportLogoResponse {
  files: ExportedFile[];
}

export async function exportLogo(params: ExportLogoRequest): Promise<ExportLogoResponse> {
  return request<ExportLogoResponse>("/api/logogen/export", {
    method: "POST",
    body: JSON.stringify(params),
  });
}

// ── Name Generation ──────────────────────────────────────────────

export interface GenerateNamesRequest {
  description: string;
  vibes: string[];
  count: number;
}

export interface DomainResult {
  tld: string;
  domain: string;
  available: boolean | null;
}

export interface NameCandidate {
  name: string;
  tagline: string | null;
  reasoning: string;
  status: string;
  domains: DomainResult[];
  npm_available: boolean | null;
  pypi_available: boolean | null;
  github_available: boolean | null;
  negative_associations: string[];
}

export interface GenerateNamesResponse {
  candidates: NameCandidate[];
}

export async function generateNames(params: GenerateNamesRequest): Promise<GenerateNamesResponse> {
  return request<GenerateNamesResponse>("/api/namegen", {
    method: "POST",
    body: JSON.stringify(params),
  });
}

// ── North Star ───────────────────────────────────────────────────

export interface PhaseInfo {
  id: number;
  name: string;
  description: string;
  document_count: number;
}

export interface NorthStarPhasesResponse {
  phases: PhaseInfo[];
  total: number;
}

export async function getNorthStarPhases(): Promise<NorthStarPhasesResponse> {
  return request<NorthStarPhasesResponse>("/api/northstar/phases");
}

export interface ExecutePhaseRequest {
  product_name: string;
  product_description: string;
  phase_id: number;
  previous_context?: string;
}

export interface DocumentResult {
  title: string;
  filename: string;
  doc_type: string;
}

export interface ExecutePhaseResponse {
  phase_id: number;
  phase_name: string;
  status: string;
  output: string;
  documents: DocumentResult[];
}

export async function executeNorthStarPhase(params: ExecutePhaseRequest): Promise<ExecutePhaseResponse> {
  return request<ExecutePhaseResponse>("/api/northstar/phase", {
    method: "POST",
    body: JSON.stringify(params),
  });
}

// ── Metrics ──────────────────────────────────────────────────────

export async function getSpendingSummary(): Promise<import("../store/observability").SpendingSummary> {
  return request("/api/metrics");
}

export async function waitForServer(
  timeoutMs: number = 10000,
  intervalMs: number = 500,
): Promise<boolean> {
  const deadline = Date.now() + timeoutMs;
  while (Date.now() < deadline) {
    try {
      await checkHealth();
      return true;
    } catch {
      await new Promise((resolve) => setTimeout(resolve, intervalMs));
    }
  }
  return false;
}
