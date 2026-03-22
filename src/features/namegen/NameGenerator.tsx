import React, { useState, useCallback } from "react";
import { generateNames } from "../../lib/api";
import type { NameCandidate } from "../../lib/api";
import { ErrorDisplay } from "../shared/ErrorDisplay";

// ── Constants ────────────────────────────────────────────────────────

const VIBE_OPTIONS = [
  "modern",
  "playful",
  "enterprise",
  "minimal",
  "bold",
  "friendly",
  "technical",
  "abstract",
  "nature",
  "futuristic",
] as const;

type Vibe = (typeof VIBE_OPTIONS)[number];

const STATUS_STYLES: Record<
  string,
  { bg: string; text: string; label: string }
> = {
  all_clear: {
    bg: "bg-green-100",
    text: "text-green-800",
    label: "All Clear",
  },
  partial: { bg: "bg-yellow-100", text: "text-yellow-800", label: "Partial" },
  conflicted: { bg: "bg-red-100", text: "text-red-800", label: "Conflicted" },
  pending: { bg: "bg-gray-100", text: "text-gray-600", label: "Pending" },
};

// ── Sub-components ───────────────────────────────────────────────────

function AvailabilityDot({
  available,
}: {
  available: boolean | null;
}): React.ReactElement {
  if (available === true) {
    return (
      <span
        className="inline-block w-3 h-3 rounded-full bg-green-500"
        title="Available"
      />
    );
  }
  if (available === false) {
    return (
      <span
        className="inline-block w-3 h-3 rounded-full bg-red-500"
        title="Taken"
      />
    );
  }
  return (
    <span
      className="inline-block w-3 h-3 rounded-full bg-gray-400"
      title="Unknown"
    />
  );
}

// ── Main Component ───────────────────────────────────────────────────

export const NameGenerator: React.FC = () => {
  const [description, setDescription] = useState("");
  const [selectedVibes, setSelectedVibes] = useState<Set<Vibe>>(new Set());
  const [candidates, setCandidates] = useState<NameCandidate[]>([]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [selectedName, setSelectedName] = useState<string | null>(null);

  const toggleVibe = useCallback((vibe: Vibe) => {
    setSelectedVibes((prev) => {
      const next = new Set(prev);
      if (next.has(vibe)) {
        next.delete(vibe);
      } else {
        next.add(vibe);
      }
      return next;
    });
  }, []);

  const generate = useCallback(async () => {
    if (!description.trim()) return;

    setLoading(true);
    setError(null);
    setCandidates([]);
    setSelectedName(null);

    try {
      const data = await generateNames({
        description: description.trim(),
        vibes: Array.from(selectedVibes),
        count: 20,
      });
      setCandidates(data.candidates);
    } catch (err) {
      setError(err instanceof Error ? err.message : "Unknown error");
    } finally {
      setLoading(false);
    }
  }, [description, selectedVibes]);

  return (
    <div className="max-w-6xl mx-auto p-6 space-y-6">
      <h2 className="text-2xl font-bold text-gray-900">Name Generator</h2>

      {/* Description Input */}
      <div>
        <label
          htmlFor="namegen-description"
          className="block text-sm font-medium text-gray-700 mb-1"
        >
          Project Description
        </label>
        <textarea
          id="namegen-description"
          className="w-full border border-gray-300 rounded-lg p-3 text-sm focus:ring-2 focus:ring-blue-500 focus:border-blue-500"
          rows={3}
          placeholder="Describe your project in a few sentences..."
          value={description}
          onChange={(e) => setDescription(e.target.value)}
        />
      </div>

      {/* Vibe Selector */}
      <div>
        <span className="block text-sm font-medium text-gray-700 mb-2">
          Vibes
        </span>
        <div className="flex flex-wrap gap-2">
          {VIBE_OPTIONS.map((vibe) => (
            <button
              key={vibe}
              type="button"
              onClick={() => toggleVibe(vibe)}
              className={`px-3 py-1 rounded-full text-sm font-medium border transition-colors ${
                selectedVibes.has(vibe)
                  ? "bg-blue-600 text-white border-blue-600"
                  : "bg-white text-gray-700 border-gray-300 hover:border-blue-400"
              }`}
            >
              {vibe}
            </button>
          ))}
        </div>
      </div>

      {/* Generate Button */}
      <button
        type="button"
        onClick={generate}
        disabled={loading || !description.trim()}
        className="px-6 py-2 bg-blue-600 text-white font-medium rounded-lg hover:bg-blue-700 disabled:opacity-50 disabled:cursor-not-allowed transition-colors"
      >
        {loading ? "Generating..." : "Generate Names"}
      </button>

      <ErrorDisplay message={error} />

      {/* Results Table */}
      {candidates.length > 0 && (
        <div className="border border-gray-200 rounded-lg overflow-hidden">
          <div className="overflow-x-auto">
            <table className="w-full text-sm">
              <thead className="bg-gray-50 border-b border-gray-200">
                <tr>
                  <th className="px-4 py-3 text-left font-medium text-gray-700">
                    Name
                  </th>
                  <th className="px-4 py-3 text-left font-medium text-gray-700">
                    Status
                  </th>
                  <th className="px-4 py-3 text-left font-medium text-gray-700">
                    Domains
                  </th>
                  <th className="px-4 py-3 text-center font-medium text-gray-700">
                    npm
                  </th>
                  <th className="px-4 py-3 text-center font-medium text-gray-700">
                    PyPI
                  </th>
                  <th className="px-4 py-3 text-center font-medium text-gray-700">
                    GitHub
                  </th>
                  <th className="px-4 py-3 text-right font-medium text-gray-700">
                    Action
                  </th>
                </tr>
              </thead>
              <tbody className="divide-y divide-gray-100">
                {candidates.map((candidate) => {
                  const style = STATUS_STYLES[candidate.status] ??
                    STATUS_STYLES.pending;
                  const isConflicted = candidate.status === "conflicted";
                  const isSelected = selectedName === candidate.name;

                  return (
                    <React.Fragment key={candidate.name}>
                      <tr
                        className={`${isConflicted ? "opacity-50" : ""} ${isSelected ? "bg-blue-50" : "hover:bg-gray-50"} transition-colors`}
                      >
                        <td className="px-4 py-3">
                          <div
                            className={`font-medium text-gray-900 ${isConflicted ? "line-through" : ""}`}
                          >
                            {candidate.name}
                          </div>
                          {candidate.tagline && (
                            <div className="text-xs text-gray-500 mt-0.5">
                              {candidate.tagline}
                            </div>
                          )}
                        </td>
                        <td className="px-4 py-3">
                          <span
                            className={`inline-flex items-center px-2 py-0.5 rounded text-xs font-medium ${style.bg} ${style.text}`}
                          >
                            {style.label}
                          </span>
                        </td>
                        <td className="px-4 py-3">
                          <div className="flex gap-2">
                            {candidate.domains.map((d) => (
                              <span
                                key={d.domain}
                                className="inline-flex items-center gap-1 text-xs text-gray-600"
                              >
                                <AvailabilityDot available={d.available} />
                                <span>.{d.tld}</span>
                              </span>
                            ))}
                          </div>
                        </td>
                        <td className="px-4 py-3 text-center">
                          <AvailabilityDot available={candidate.npm_available} />
                        </td>
                        <td className="px-4 py-3 text-center">
                          <AvailabilityDot available={candidate.pypi_available} />
                        </td>
                        <td className="px-4 py-3 text-center">
                          <AvailabilityDot available={candidate.github_available} />
                        </td>
                        <td className="px-4 py-3 text-right">
                          <button
                            type="button"
                            onClick={() => setSelectedName(candidate.name)}
                            disabled={isConflicted}
                            className="text-xs px-3 py-1 rounded border border-blue-300 text-blue-700 hover:bg-blue-50 disabled:opacity-30 disabled:cursor-not-allowed transition-colors"
                          >
                            Select
                          </button>
                        </td>
                      </tr>
                      {candidate.negative_associations.length > 0 && (
                        <tr className="bg-red-50">
                          <td colSpan={7} className="px-4 py-2">
                            <div className="flex items-start gap-2 text-xs text-red-700">
                              <span className="font-medium shrink-0">
                                Warnings:
                              </span>
                              <span>
                                {candidate.negative_associations.join("; ")}
                              </span>
                            </div>
                          </td>
                        </tr>
                      )}
                    </React.Fragment>
                  );
                })}
              </tbody>
            </table>
          </div>
        </div>
      )}

      {selectedName && (
        <div className="flex items-center gap-4 p-4 bg-blue-50 border border-blue-200 rounded-lg">
          <span className="text-sm text-blue-800">
            Selected: <strong>{selectedName}</strong>
          </span>
          <button
            type="button"
            className="ml-auto px-4 py-2 bg-blue-600 text-white font-medium rounded-lg hover:bg-blue-700 transition-colors"
          >
            Apply to Project
          </button>
        </div>
      )}
    </div>
  );
};
