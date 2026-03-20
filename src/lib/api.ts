import type { Task, CreateTask } from "../types/task";
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
