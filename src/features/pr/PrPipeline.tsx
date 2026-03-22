import React, { useState } from "react";
import { createPr } from "../../lib/api";
import type { PrStepResult } from "../../lib/api";
import { ErrorDisplay } from "../shared/ErrorDisplay";

export interface PrPipelineProps {
  taskId: number;
  taskTitle: string;
  branch: string;
}

const STEP_STYLES: Record<string, { dot: string; text: string }> = {
  pending: { dot: "bg-gray-500", text: "text-gray-400" },
  running: { dot: "bg-blue-500 animate-pulse", text: "text-blue-400" },
  passed: { dot: "bg-green-500", text: "text-green-400" },
  failed: { dot: "bg-red-500", text: "text-red-400" },
  skipped: { dot: "bg-gray-600", text: "text-gray-500" },
};

export const PrPipeline: React.FC<PrPipelineProps> = ({
  taskId,
  taskTitle,
  branch,
}) => {
  const [steps, setSteps] = useState<PrStepResult[]>([]);
  const [prUrl, setPrUrl] = useState<string | null>(null);
  const [running, setRunning] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [baseBranch, setBaseBranch] = useState("main");

  const handleCreatePr = async () => {
    setRunning(true);
    setError(null);
    setSteps([]);
    setPrUrl(null);

    try {
      const data = await createPr(taskId, {
        base_branch: baseBranch,
        auto_commit_message: true,
        run_gates: true,
      });

      setSteps(data.steps ?? []);
      setPrUrl(data.pr_url ?? null);

      if (!data.success) {
        setError("PR pipeline completed with failures.");
      }
    } catch (e) {
      setError(e instanceof Error ? e.message : "Unknown error");
    } finally {
      setRunning(false);
    }
  };

  return (
    <div className="space-y-4" data-testid="pr-pipeline">
      <div className="text-sm text-gray-400">
        Branch: <span className="text-gray-200 font-mono">{branch}</span>
        {" / "}
        Task: <span className="text-gray-200">{taskTitle}</span>
      </div>

      <div className="flex items-center gap-2">
        <label className="text-sm text-gray-400" htmlFor="base-branch">
          Base branch:
        </label>
        <input
          id="base-branch"
          type="text"
          className="px-2 py-1 text-sm rounded bg-gray-800 border border-gray-600 text-gray-200 focus:outline-none focus:border-blue-500"
          value={baseBranch}
          onChange={(e) => setBaseBranch(e.target.value)}
          disabled={running}
          data-testid="base-branch-input"
        />
        <button
          onClick={handleCreatePr}
          disabled={running}
          className="px-3 py-1 text-sm font-medium rounded bg-blue-600 text-white hover:bg-blue-500 disabled:opacity-50 disabled:cursor-not-allowed"
          data-testid="create-pr-btn"
        >
          {running ? "Creating..." : "Create PR"}
        </button>
      </div>

      {steps.length > 0 && (
        <div className="space-y-1" data-testid="pr-steps">
          {steps.map((step, idx) => {
            const style = STEP_STYLES[step.status] ?? STEP_STYLES.pending;
            return (
              <details key={idx} className="group" data-testid={`pr-step-${idx}`}>
                <summary className="flex items-center gap-2 cursor-pointer text-sm list-none">
                  <span
                    className={`w-2.5 h-2.5 rounded-full ${style.dot}`}
                    data-testid={`pr-step-dot-${idx}`}
                  />
                  <span className="flex-1">{step.name}</span>
                  <span className={`text-xs font-semibold ${style.text}`}>
                    {step.status}
                  </span>
                </summary>
                {step.output && (
                  <pre
                    className="mt-1 ml-5 p-2 rounded bg-gray-900 text-xs text-gray-300 overflow-x-auto whitespace-pre-wrap"
                    data-testid={`pr-step-output-${idx}`}
                  >
                    {step.output}
                  </pre>
                )}
              </details>
            );
          })}
        </div>
      )}

      <ErrorDisplay message={error} testId="pr-error" variant="dark" />

      {prUrl && (
        <div
          className="p-3 rounded bg-green-900/30 border border-green-700 text-sm text-green-300"
          data-testid="pr-success"
        >
          PR created:{" "}
          <a
            href={prUrl}
            target="_blank"
            rel="noopener noreferrer"
            className="underline text-green-200 hover:text-green-100"
          >
            {prUrl}
          </a>
        </div>
      )}
    </div>
  );
};
