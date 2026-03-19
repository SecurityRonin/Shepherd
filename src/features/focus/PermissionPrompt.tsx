import React, { useState, useCallback } from "react";
import { useStore } from "../../store";
import { approveTask } from "../../lib/api";

interface PermissionPromptProps {
  taskId: number;
}

export const PermissionPrompt: React.FC<PermissionPromptProps> = ({ taskId }) => {
  const wsClient = useStore((s) => s.wsClient);
  const pendingPermissions = useStore((s) => s.pendingPermissions);
  const taskPermissions = pendingPermissions.filter((p) => p.task_id === taskId);

  const [showCustomInput, setShowCustomInput] = useState(false);
  const [customText, setCustomText] = useState("");
  const [isApproving, setIsApproving] = useState(false);

  const latestPermission = taskPermissions[taskPermissions.length - 1];

  const handleApprove = useCallback(async () => {
    setIsApproving(true);
    try {
      await approveTask(taskId);
    } catch {
      // Error handled upstream
    } finally {
      setIsApproving(false);
    }
  }, [taskId]);

  const handleApproveAll = useCallback(async () => {
    setIsApproving(true);
    try {
      await approveTask(taskId);
    } catch {
      // Error handled upstream
    } finally {
      setIsApproving(false);
    }
  }, [taskId]);

  const handleCustomSubmit = useCallback(() => {
    if (!customText.trim()) return;
    if (wsClient) {
      wsClient.send({
        type: "terminal_input",
        data: { task_id: taskId, data: customText + "\n" },
      });
    }
    setCustomText("");
    setShowCustomInput(false);
  }, [customText, taskId, wsClient]);

  if (!latestPermission) return null;

  // Parse tool_args for display
  let toolArgsDisplay: string;
  try {
    const parsed = JSON.parse(latestPermission.tool_args);
    toolArgsDisplay = JSON.stringify(parsed, null, 2);
  } catch {
    toolArgsDisplay = latestPermission.tool_args;
  }

  return (
    <div className="border-t-2 border-orange-500/50 bg-orange-950/20 p-3">
      {/* Permission question */}
      <div className="mb-3">
        <div className="flex items-center gap-2 mb-1.5">
          <span className="w-2 h-2 rounded-full bg-orange-500 animate-pulse" />
          <span className="text-xs font-semibold text-orange-400 uppercase tracking-wider">
            Permission Required
          </span>
        </div>
        <div className="text-sm text-shepherd-text font-medium mb-1" data-testid="permission-tool-name">
          {latestPermission.tool_name}
        </div>
        {toolArgsDisplay && (
          <pre className="text-[11px] text-shepherd-muted font-mono bg-shepherd-bg/50 rounded p-2 overflow-x-auto max-h-24 overflow-y-auto">
            {toolArgsDisplay}
          </pre>
        )}
      </div>

      {/* Action buttons */}
      <div className="flex items-center gap-2">
        <button
          onClick={handleApprove}
          disabled={isApproving}
          className="px-3 py-1.5 text-xs font-medium rounded bg-green-600 hover:bg-green-500 text-white transition-colors disabled:opacity-50"
          data-testid="approve-button"
        >
          Approve
        </button>
        <button
          onClick={handleApproveAll}
          disabled={isApproving}
          className="px-3 py-1.5 text-xs font-medium rounded bg-shepherd-border hover:bg-shepherd-muted/30 text-shepherd-text transition-colors disabled:opacity-50"
          data-testid="approve-all-button"
        >
          Approve All
        </button>
        <button
          onClick={() => setShowCustomInput(!showCustomInput)}
          className="px-3 py-1.5 text-xs font-medium rounded bg-shepherd-border hover:bg-shepherd-muted/30 text-shepherd-text transition-colors"
          data-testid="custom-toggle-button"
        >
          Custom...
        </button>

        {/* Keyboard hints */}
        <div className="ml-auto flex items-center gap-3 text-[10px] text-shepherd-muted">
          <span>
            <kbd className="px-1 py-0.5 rounded bg-shepherd-border font-mono">&#8984;&#9166;</kbd>{" "}
            approve
          </span>
          <span>
            <kbd className="px-1 py-0.5 rounded bg-shepherd-border font-mono">&#8984;&#8679;&#9166;</kbd>{" "}
            approve all
          </span>
        </div>
      </div>

      {/* Custom input */}
      {showCustomInput && (
        <div className="mt-2 flex items-center gap-2" data-testid="custom-input-area">
          <input
            type="text"
            value={customText}
            onChange={(e) => setCustomText(e.target.value)}
            onKeyDown={(e) => {
              if (e.key === "Enter") {
                e.preventDefault();
                handleCustomSubmit();
              }
              if (e.key === "Escape") {
                setShowCustomInput(false);
              }
            }}
            placeholder="Type custom response..."
            className="flex-1 px-3 py-1.5 text-xs rounded bg-shepherd-bg border border-shepherd-border text-shepherd-text placeholder:text-shepherd-muted focus:outline-none focus:border-shepherd-accent"
            autoFocus
            data-testid="custom-input"
          />
          <button
            onClick={handleCustomSubmit}
            className="px-3 py-1.5 text-xs font-medium rounded bg-shepherd-accent hover:bg-shepherd-accent/80 text-white transition-colors"
          >
            Send
          </button>
        </div>
      )}
    </div>
  );
};
