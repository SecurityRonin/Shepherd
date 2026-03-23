import type { Task } from "../types/task";

export interface ExportableTask {
  id: number;
  title: string;
  prompt: string;
  agent_id: string;
  status: string;
  branch: string;
  repo_path: string;
  isolation_mode: string;
  created_at: string;
  updated_at: string;
  summary: string;
  files_changed: number;
  diffs: { file_path: string; language: string }[];
  gate_results: { gate: string; passed: boolean }[];
}

export function exportTasksAsJson(tasks: Task[]): string {
  const exportable: ExportableTask[] = tasks.map((t) => ({
    id: t.id,
    title: t.title,
    prompt: t.prompt,
    agent_id: t.agent_id,
    status: t.status,
    branch: t.branch,
    repo_path: t.repo_path,
    isolation_mode: t.isolation_mode,
    created_at: t.created_at,
    updated_at: t.updated_at,
    summary: t.summary ?? "",
    files_changed: t.diffs?.length ?? 0,
    diffs:
      t.diffs?.map((d) => ({
        file_path: d.file_path,
        language: d.language,
      })) ?? [],
    gate_results: t.gate_results ?? [],
  }));
  return JSON.stringify(
    { exported_at: new Date().toISOString(), tasks: exportable },
    null,
    2,
  );
}

export function exportTasksAsCsv(tasks: Task[]): string {
  const headers = [
    "id",
    "title",
    "agent_id",
    "status",
    "branch",
    "repo_path",
    "isolation_mode",
    "created_at",
    "updated_at",
    "summary",
    "files_changed",
  ];
  const rows = tasks.map((t) =>
    [
      t.id,
      escapeCsvField(t.title),
      t.agent_id,
      t.status,
      escapeCsvField(t.branch),
      escapeCsvField(t.repo_path),
      t.isolation_mode,
      t.created_at,
      t.updated_at,
      escapeCsvField(t.summary ?? ""),
      t.diffs?.length ?? 0,
    ].join(","),
  );
  return [headers.join(","), ...rows].join("\n");
}

function escapeCsvField(value: string): string {
  if (value.includes(",") || value.includes('"') || value.includes("\n")) {
    return `"${value.replace(/"/g, '""')}"`;
  }
  return value;
}

export interface MetricsExport {
  exported_at: string;
  total_cost_usd: number;
  total_tasks: number;
  cost_by_agent: Record<string, number>;
  cost_by_day: { date: string; cost: number }[];
}

export function exportMetricsAsJson(metrics: {
  total_cost_usd: number;
  total_tasks: number;
  cost_by_agent: Record<string, number>;
  cost_by_day: { date: string; cost: number }[];
}): string {
  return JSON.stringify(
    {
      exported_at: new Date().toISOString(),
      ...metrics,
    },
    null,
    2,
  );
}

export function triggerDownload(
  content: string,
  filename: string,
  mimeType: string,
): void {
  const blob = new Blob([content], { type: mimeType });
  const url = URL.createObjectURL(blob);
  const a = document.createElement("a");
  a.href = url;
  a.download = filename;
  a.click();
  URL.revokeObjectURL(url);
}
