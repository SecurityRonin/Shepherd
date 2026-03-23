import React from "react";
import type { SessionState } from "../../types/task";

export interface ResumePromptProps {
  sessions: SessionState[];
  onResume: (taskId: number) => void;
  onFresh: (taskId: number) => void;
  onDismiss: (taskId: number) => void;
}

function truncatePrompt(prompt: string): string {
  if (prompt.length <= 80) return prompt;
  return prompt.slice(0, 80) + "...";
}

export const ResumePrompt: React.FC<ResumePromptProps> = ({
  sessions,
  onResume,
  onFresh,
  onDismiss,
}) => {
  if (sessions.length === 0) return null;

  return (
    <div data-testid="resume-prompt" className="space-y-2">
      {sessions.map((session) => (
        <div
          key={session.task_id}
          className="bg-shepherd-surface border border-shepherd-border rounded-lg p-4 flex items-start justify-between gap-4"
        >
          <div className="flex-1 min-w-0">
            <p className="text-sm font-medium text-shepherd-text truncate">
              {truncatePrompt(session.last_prompt)}
            </p>
            <p className="text-xs text-shepherd-muted-foreground mt-1">
              <span>{session.working_dir}</span>
              <span className="mx-2">&#183;</span>
              <span>{new Date(session.saved_at).toLocaleString()}</span>
            </p>
          </div>
          <div className="flex gap-2 shrink-0">
            <button
              data-testid="resume-btn"
              onClick={() => onResume(session.task_id)}
              className="px-3 py-1.5 text-xs font-medium rounded bg-shepherd-accent text-white hover:bg-shepherd-accent/90"
            >
              Resume
            </button>
            <button
              data-testid="fresh-btn"
              onClick={() => onFresh(session.task_id)}
              className="px-3 py-1.5 text-xs font-medium rounded bg-shepherd-surface border border-shepherd-border hover:bg-shepherd-muted"
            >
              Start Fresh
            </button>
            <button
              data-testid="dismiss-btn"
              onClick={() => onDismiss(session.task_id)}
              className="px-3 py-1.5 text-xs font-medium rounded text-shepherd-muted-foreground hover:text-shepherd-text"
            >
              Dismiss
            </button>
          </div>
        </div>
      ))}
    </div>
  );
};
