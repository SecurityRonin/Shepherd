import React, { useState, useEffect, useCallback } from "react";

// ── Types ────────────────────────────────────────────────────────────

interface PhaseInfo {
  id: number;
  name: string;
  description: string;
  document_count: number;
}

interface DocumentResponse {
  title: string;
  filename: string;
  doc_type: string;
}

interface ExecutePhaseResponse {
  phase_id: number;
  phase_name: string;
  status: string;
  output: string;
  documents: DocumentResponse[];
}

interface PhaseState {
  info: PhaseInfo;
  status: "pending" | "running" | "completed" | "failed";
  result: ExecutePhaseResponse | null;
}

// ── Status Dot ───────────────────────────────────────────────────────

function StatusDot({
  status,
}: {
  status: PhaseState["status"];
}): React.ReactElement {
  const styles: Record<PhaseState["status"], string> = {
    pending: "bg-gray-400",
    running: "bg-blue-500 animate-pulse",
    completed: "bg-green-500",
    failed: "bg-red-500",
  };

  return (
    <span
      className={`inline-block w-3 h-3 rounded-full ${styles[status]}`}
      title={status}
    />
  );
}

// ── Component ────────────────────────────────────────────────────────

export const NorthStarWizard: React.FC = () => {
  const [productName, setProductName] = useState("");
  const [productDescription, setProductDescription] = useState("");
  const [phases, setPhases] = useState<PhaseState[]>([]);
  const [analyzing, setAnalyzing] = useState(false);
  const [error, setError] = useState<string | null>(null);

  // Fetch phases on mount
  useEffect(() => {
    const fetchPhases = async () => {
      try {
        const response = await fetch("/api/northstar/phases");
        if (!response.ok) throw new Error(`HTTP ${response.status}`);
        const data: { phases: PhaseInfo[]; total: number } =
          await response.json();
        setPhases(
          data.phases.map((info) => ({
            info,
            status: "pending" as const,
            result: null,
          })),
        );
      } catch (err) {
        setError(
          err instanceof Error ? err.message : "Failed to load phases",
        );
      }
    };

    fetchPhases();
  }, []);

  const startAnalysis = useCallback(async () => {
    if (!productName.trim() || !productDescription.trim() || analyzing) return;

    setAnalyzing(true);
    setError(null);

    // Reset all phases to pending
    setPhases((prev) =>
      prev.map((p) => ({ ...p, status: "pending" as const, result: null })),
    );

    let previousContext = "";

    for (let i = 0; i < phases.length; i++) {
      const phase = phases[i];

      // Mark current phase as running
      setPhases((prev) =>
        prev.map((p, idx) =>
          idx === i ? { ...p, status: "running" as const } : p,
        ),
      );

      try {
        const response = await fetch("/api/northstar/phase", {
          method: "POST",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify({
            product_name: productName.trim(),
            product_description: productDescription.trim(),
            phase_id: phase.info.id,
            previous_context: previousContext || undefined,
          }),
        });

        if (!response.ok) {
          const body = await response.json().catch(() => ({}));
          throw new Error(
            (body as Record<string, string>).error ||
              `HTTP ${response.status}`,
          );
        }

        const result: ExecutePhaseResponse = await response.json();

        // Mark phase as completed
        setPhases((prev) =>
          prev.map((p, idx) =>
            idx === i
              ? { ...p, status: "completed" as const, result }
              : p,
          ),
        );

        // Build context for next phase (first 2000 chars)
        const snippet = result.output.slice(0, 2000);
        previousContext += `\n\n## Phase ${result.phase_id}: ${result.phase_name}\n${snippet}`;
      } catch (err) {
        // Mark phase as failed
        setPhases((prev) =>
          prev.map((p, idx) =>
            idx === i ? { ...p, status: "failed" as const } : p,
          ),
        );
        setError(
          err instanceof Error ? err.message : `Phase ${phase.info.id} failed`,
        );
        break;
      }
    }

    setAnalyzing(false);
  }, [productName, productDescription, analyzing, phases]);

  const completedPhases = phases.filter((p) => p.status === "completed");
  const totalDocs = completedPhases.reduce(
    (sum, p) => sum + (p.result?.documents.length ?? 0),
    0,
  );
  const allDone =
    phases.length > 0 && completedPhases.length === phases.length;

  return (
    <div className="max-w-4xl mx-auto p-6 space-y-8">
      <h2 className="text-2xl font-bold text-gray-900">
        North Star PMF Analysis
      </h2>

      {/* Input Form */}
      <div className="space-y-4">
        <div className="space-y-2">
          <label className="block text-sm font-medium text-gray-700">
            Product Name
          </label>
          <input
            type="text"
            value={productName}
            onChange={(e) => setProductName(e.target.value)}
            placeholder="Enter product name..."
            disabled={analyzing}
            className="w-full px-4 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-blue-500 focus:border-blue-500 disabled:opacity-50"
          />
        </div>

        <div className="space-y-2">
          <label className="block text-sm font-medium text-gray-700">
            Product Description
          </label>
          <textarea
            value={productDescription}
            onChange={(e) => setProductDescription(e.target.value)}
            placeholder="Describe your product, target market, and key features..."
            rows={4}
            disabled={analyzing}
            className="w-full px-4 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-blue-500 focus:border-blue-500 disabled:opacity-50"
          />
        </div>

        <button
          onClick={startAnalysis}
          disabled={
            !productName.trim() ||
            !productDescription.trim() ||
            analyzing ||
            phases.length === 0
          }
          className="w-full py-3 px-4 bg-blue-600 text-white rounded-lg font-medium hover:bg-blue-700 disabled:opacity-50 disabled:cursor-not-allowed transition-colors"
        >
          {analyzing
            ? "Analyzing..."
            : `Start Analysis (${phases.length} Phases)`}
        </button>
      </div>

      {/* Error */}
      {error && (
        <div className="p-4 bg-red-50 border border-red-200 rounded-lg text-red-700 text-sm">
          {error}
        </div>
      )}

      {/* Progress Counter */}
      {(analyzing || completedPhases.length > 0) && (
        <div className="text-sm text-gray-600">
          {completedPhases.length}/{phases.length} phases | {totalDocs} documents
        </div>
      )}

      {/* Phase Progress */}
      {phases.length > 0 &&
        (analyzing || completedPhases.length > 0) && (
          <div className="space-y-3">
            <h3 className="text-lg font-semibold text-gray-900">
              Phase Progress
            </h3>
            <ul className="space-y-2">
              {phases.map((phase) => (
                <li
                  key={phase.info.id}
                  className="flex items-start gap-3 p-3 rounded-lg bg-gray-50"
                >
                  <StatusDot status={phase.status} />
                  <div className="flex-1">
                    <div className="flex items-center justify-between">
                      <span className="font-medium text-gray-900 text-sm">
                        Phase {phase.info.id}: {phase.info.name}
                      </span>
                      <span className="text-xs text-gray-500">
                        {phase.info.description}
                      </span>
                    </div>

                    {/* Document list for completed phases */}
                    {phase.status === "completed" &&
                      phase.result &&
                      phase.result.documents.length > 0 && (
                        <ul className="mt-2 space-y-1">
                          {phase.result.documents.map((doc) => (
                            <li
                              key={doc.filename}
                              className="text-xs text-gray-600 flex items-center gap-1"
                            >
                              <span className="text-green-500">&#9679;</span>
                              {doc.title} ({doc.filename})
                            </li>
                          ))}
                        </ul>
                      )}
                  </div>
                </li>
              ))}
            </ul>
          </div>
        )}

      {/* Completion Summary */}
      {allDone && (
        <div className="p-6 bg-green-50 border border-green-200 rounded-lg space-y-3">
          <h3 className="text-lg font-semibold text-green-800">
            Analysis Complete
          </h3>
          <p className="text-sm text-green-700">
            All {phases.length} phases completed successfully with {totalDocs}{" "}
            documents generated. The ai-context.yml file has been generated
            with strategic context for AI coding assistants.
          </p>
          <div className="text-xs text-green-600 font-mono">
            docs/northstar/ai-context.yml
          </div>
        </div>
      )}
    </div>
  );
};
