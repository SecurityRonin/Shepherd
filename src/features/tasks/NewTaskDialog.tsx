import React, { useState, useEffect, useCallback, useRef } from "react";
import { createTask } from "../../lib/api";
import type { CreateTask } from "../../types/task";

const AGENTS = [
  { id: "claude-code", label: "Claude Code" },
  { id: "codex-cli", label: "Codex CLI" },
  { id: "opencode", label: "OpenCode" },
  { id: "gemini-cli", label: "Gemini CLI" },
  { id: "aider", label: "Aider" },
] as const;

const ISOLATION_MODES = [
  { id: "worktree", label: "Worktree" },
  { id: "docker", label: "Docker" },
  { id: "local", label: "Local" },
] as const;

interface NewTaskDialogProps {
  isOpen: boolean;
  onClose: () => void;
}

export const NewTaskDialog: React.FC<NewTaskDialogProps> = ({
  isOpen,
  onClose,
}) => {
  const [prompt, setPrompt] = useState("");
  const [agentId, setAgentId] = useState<string>("claude-code");
  const [repoPath, setRepoPath] = useState(".");
  const [isolationMode, setIsolationMode] = useState("worktree");
  const [initialMessage, setInitialMessage] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [submitting, setSubmitting] = useState(false);
  const promptRef = useRef<HTMLTextAreaElement>(null);

  // Reset form when dialog opens
  useEffect(() => {
    if (isOpen) {
      setPrompt("");
      setAgentId("claude-code");
      setRepoPath(".");
      setIsolationMode("worktree");
      setInitialMessage("");
      setError(null);
      setSubmitting(false);
      setTimeout(() => promptRef.current?.focus(), 0);
    }
  }, [isOpen]);

  const handleSubmit = useCallback(async () => {
    if (!prompt.trim()) {
      setError("Task prompt is required");
      return;
    }

    setError(null);
    setSubmitting(true);

    try {
      const taskData: CreateTask = {
        title: prompt.trim(),
        prompt: initialMessage.trim() || undefined,
        agent_id: agentId,
        repo_path: repoPath.trim() || ".",
        isolation_mode: isolationMode,
      };
      await createTask(taskData);
      onClose();
    } catch (err) {
      setError(
        err instanceof Error ? err.message : "Failed to create task",
      );
    } finally {
      setSubmitting(false);
    }
  }, [prompt, agentId, repoPath, isolationMode, initialMessage, onClose]);

  // Keyboard shortcuts
  const handleKeyDown = useCallback(
    (e: React.KeyboardEvent) => {
      if (e.key === "Escape") {
        e.preventDefault();
        onClose();
      }
      if ((e.metaKey || e.ctrlKey) && e.key === "Enter") {
        e.preventDefault();
        handleSubmit();
      }
    },
    [onClose, handleSubmit],
  );

  if (!isOpen) return null;

  return (
    <div
      className="fixed inset-0 z-50 flex items-center justify-center"
      onClick={onClose}
    >
      {/* Backdrop */}
      <div className="fixed inset-0 bg-black/50" />

      {/* Dialog */}
      <div
        className="relative z-10 w-full max-w-lg rounded-xl border border-zinc-700 bg-zinc-900 p-6 shadow-2xl"
        onClick={(e) => e.stopPropagation()}
        onKeyDown={handleKeyDown}
      >
        <h2 className="mb-4 text-lg font-semibold text-white">New Task</h2>

        {/* Error display */}
        {error && (
          <div className="mb-4 rounded-lg border border-red-800 bg-red-900/50 px-4 py-2 text-sm text-red-300">
            {error}
          </div>
        )}

        {/* Task Prompt */}
        <div className="mb-4">
          <label className="mb-1 block text-sm font-medium text-zinc-300">
            Task Prompt
          </label>
          <textarea
            ref={promptRef}
            value={prompt}
            onChange={(e) => setPrompt(e.target.value)}
            placeholder="Describe what you want the agent to do..."
            rows={3}
            className="w-full rounded-lg border border-zinc-700 bg-zinc-800 px-3 py-2 text-sm text-white placeholder-zinc-500 outline-none focus:border-blue-500"
          />
        </div>

        {/* Agent selector */}
        <div className="mb-4">
          <label className="mb-1 block text-sm font-medium text-zinc-300">
            Agent
          </label>
          <div className="grid grid-cols-5 gap-2">
            {AGENTS.map((agent) => (
              <button
                key={agent.id}
                type="button"
                className={`rounded-lg border px-2 py-1.5 text-xs font-medium transition-colors ${
                  agentId === agent.id
                    ? "border-blue-500 bg-blue-500/20 text-blue-300"
                    : "border-zinc-700 bg-zinc-800 text-zinc-400 hover:border-zinc-600"
                }`}
                onClick={() => setAgentId(agent.id)}
              >
                {agent.label}
              </button>
            ))}
          </div>
        </div>

        {/* Repo Path */}
        <div className="mb-4">
          <label className="mb-1 block text-sm font-medium text-zinc-300">
            Repo Path
          </label>
          <input
            type="text"
            value={repoPath}
            onChange={(e) => setRepoPath(e.target.value)}
            placeholder="."
            className="w-full rounded-lg border border-zinc-700 bg-zinc-800 px-3 py-2 text-sm text-white placeholder-zinc-500 outline-none focus:border-blue-500"
          />
        </div>

        {/* Isolation Mode */}
        <div className="mb-4">
          <label className="mb-1 block text-sm font-medium text-zinc-300">
            Isolation Mode
          </label>
          <div className="flex gap-4">
            {ISOLATION_MODES.map((mode) => (
              <label
                key={mode.id}
                className="flex cursor-pointer items-center gap-2 text-sm text-zinc-300"
              >
                <input
                  type="radio"
                  name="isolation-mode"
                  value={mode.id}
                  checked={isolationMode === mode.id}
                  onChange={() => setIsolationMode(mode.id)}
                  className="text-blue-500"
                />
                {mode.label}
              </label>
            ))}
          </div>
        </div>

        {/* Initial Message (optional) */}
        <div className="mb-6">
          <label className="mb-1 block text-sm font-medium text-zinc-300">
            Initial Message{" "}
            <span className="text-zinc-500">(optional)</span>
          </label>
          <textarea
            value={initialMessage}
            onChange={(e) => setInitialMessage(e.target.value)}
            placeholder="Optional initial message to the agent..."
            rows={2}
            className="w-full rounded-lg border border-zinc-700 bg-zinc-800 px-3 py-2 text-sm text-white placeholder-zinc-500 outline-none focus:border-blue-500"
          />
        </div>

        {/* Footer */}
        <div className="flex items-center justify-between">
          <span className="text-xs text-zinc-500">
            <kbd className="rounded border border-zinc-600 px-1">
              {"\u2318"} Enter
            </kbd>{" "}
            to submit
          </span>
          <div className="flex gap-2">
            <button
              type="button"
              onClick={onClose}
              className="rounded-lg border border-zinc-700 px-4 py-2 text-sm text-zinc-300 hover:bg-zinc-800"
            >
              Cancel
            </button>
            <button
              type="button"
              onClick={handleSubmit}
              disabled={submitting}
              className="rounded-lg bg-blue-600 px-4 py-2 text-sm font-medium text-white hover:bg-blue-500 disabled:opacity-50"
            >
              {submitting ? "Creating..." : "Create Task"}
            </button>
          </div>
        </div>
      </div>
    </div>
  );
};
