import React, { useState } from "react";
import type { Task } from "../../types/task";
import {
  exportTasksAsJson,
  exportTasksAsCsv,
  exportMetricsAsJson,
  triggerDownload,
} from "../../lib/export";

export interface ExportButtonProps {
  tasks: Task[];
  metrics?: {
    total_cost_usd: number;
    total_tasks: number;
    cost_by_agent: Record<string, number>;
    cost_by_day: { date: string; cost: number }[];
  };
}

export const ExportButton: React.FC<ExportButtonProps> = ({ tasks, metrics }) => {
  const [open, setOpen] = useState(false);

  function handleExportTasksJson() {
    const content = exportTasksAsJson(tasks);
    triggerDownload(content, "tasks-export.json", "application/json");
    setOpen(false);
  }

  function handleExportTasksCsv() {
    const content = exportTasksAsCsv(tasks);
    triggerDownload(content, "tasks-export.csv", "text/csv");
    setOpen(false);
  }

  function handleExportMetricsJson() {
    if (!metrics) return;
    const content = exportMetricsAsJson(metrics);
    triggerDownload(content, "metrics-export.json", "application/json");
    setOpen(false);
  }

  return (
    <div className="relative inline-block">
      <button
        data-testid="export-btn"
        onClick={() => setOpen((prev) => !prev)}
        className="flex items-center gap-1 px-3 py-1.5 text-sm bg-gray-700 text-gray-200 rounded hover:bg-gray-600 transition-colors"
      >
        Export
        <svg
          className={`w-3 h-3 transition-transform ${open ? "rotate-180" : ""}`}
          fill="none"
          stroke="currentColor"
          viewBox="0 0 24 24"
        >
          <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M19 9l-7 7-7-7" />
        </svg>
      </button>

      {open && (
        <div
          data-testid="export-menu"
          className="absolute right-0 mt-1 w-48 bg-gray-800 border border-gray-700 rounded shadow-lg z-50"
        >
          <button
            data-testid="export-tasks-json"
            onClick={handleExportTasksJson}
            className="w-full text-left px-4 py-2 text-sm text-gray-200 hover:bg-gray-700 transition-colors"
          >
            Tasks as JSON
          </button>
          <button
            data-testid="export-tasks-csv"
            onClick={handleExportTasksCsv}
            className="w-full text-left px-4 py-2 text-sm text-gray-200 hover:bg-gray-700 transition-colors"
          >
            Tasks as CSV
          </button>
          {metrics && (
            <button
              data-testid="export-metrics-json"
              onClick={handleExportMetricsJson}
              className="w-full text-left px-4 py-2 text-sm text-gray-200 hover:bg-gray-700 transition-colors"
            >
              Metrics as JSON
            </button>
          )}
        </div>
      )}
    </div>
  );
};
