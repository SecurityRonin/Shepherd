import React from "react";

export interface GateResult {
  gate_name: string;
  passed: boolean;
  output: string;
  duration_ms: number;
  gate_type: string;
}

export interface GateResultsProps {
  results: GateResult[];
  loading?: boolean;
}

const GATE_ICONS: Record<string, string> = {
  lint: "L",
  format: "F",
  type_check: "T",
  test: "X",
  security: "S",
  custom: "C",
};

export const GateResults: React.FC<GateResultsProps> = ({ results, loading }) => {
  if (loading) {
    return (
      <div className="flex items-center gap-2 p-4" data-testid="gate-loading">
        <div className="w-4 h-4 border-2 border-blue-400 border-t-transparent rounded-full animate-spin" />
        <span className="text-sm text-gray-300">Running quality gates...</span>
      </div>
    );
  }

  if (results.length === 0) {
    return null;
  }

  const allPassed = results.every((r) => r.passed);
  const passCount = results.filter((r) => r.passed).length;

  return (
    <div
      className={`rounded-lg border p-4 ${
        allPassed
          ? "bg-green-900/20 border-green-700"
          : "bg-red-900/20 border-red-700"
      }`}
      data-testid="gate-results"
    >
      <div className="text-sm font-medium mb-3" data-testid="gate-header">
        {passCount}/{results.length} gates passed
      </div>
      <div className="space-y-2">
        {results.map((result, idx) => (
          <details key={idx} className="group" data-testid={`gate-${idx}`}>
            <summary className="flex items-center gap-2 cursor-pointer text-sm list-none">
              <span
                className="inline-flex items-center justify-center w-5 h-5 rounded text-xs font-bold bg-gray-700 text-gray-200"
                data-testid={`gate-icon-${idx}`}
              >
                {GATE_ICONS[result.gate_type] ?? "?"}
              </span>
              <span className="flex-1">{result.gate_name}</span>
              <span
                className={`text-xs font-semibold ${
                  result.passed ? "text-green-400" : "text-red-400"
                }`}
              >
                {result.passed ? "PASS" : "FAIL"}
              </span>
              <span className="text-xs text-gray-500">
                {result.duration_ms}ms
              </span>
            </summary>
            {result.output && (
              <pre
                className="mt-2 p-2 rounded bg-gray-900 text-xs text-gray-300 overflow-x-auto whitespace-pre-wrap"
                data-testid={`gate-output-${idx}`}
              >
                {result.output}
              </pre>
            )}
          </details>
        ))}
      </div>
    </div>
  );
};
