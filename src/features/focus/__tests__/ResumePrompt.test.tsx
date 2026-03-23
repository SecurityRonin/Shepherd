import { describe, it, expect, vi } from "vitest";
import { render, screen, fireEvent } from "@testing-library/react";
import { ResumePrompt } from "../ResumePrompt";
import type { SessionState } from "../../../types/task";

function makeSession(overrides: Partial<SessionState> = {}): SessionState {
  return {
    task_id: 1,
    session_id: "sess-abc",
    last_prompt: "Implement the user authentication flow",
    working_dir: "/home/user/project",
    saved_at: "2026-03-23T10:00:00Z",
    ...overrides,
  };
}

describe("ResumePrompt", () => {
  const onResume = vi.fn();
  const onFresh = vi.fn();
  const onDismiss = vi.fn();

  it("renders nothing when sessions array is empty", () => {
    const { container } = render(
      <ResumePrompt sessions={[]} onResume={onResume} onFresh={onFresh} onDismiss={onDismiss} />,
    );
    expect(container.innerHTML).toBe("");
  });

  it("renders a banner for each interrupted session", () => {
    const sessions = [
      makeSession({ task_id: 1 }),
      makeSession({ task_id: 2, last_prompt: "Fix the build pipeline" }),
    ];
    render(
      <ResumePrompt sessions={sessions} onResume={onResume} onFresh={onFresh} onDismiss={onDismiss} />,
    );
    const resumeButtons = screen.getAllByTestId("resume-btn");
    expect(resumeButtons).toHaveLength(2);
  });

  it("shows truncated prompt text (first 80 chars + '...')", () => {
    const longPrompt =
      "This is a very long prompt that exceeds eighty characters and should be truncated with an ellipsis at the end";
    const sessions = [makeSession({ last_prompt: longPrompt })];
    render(
      <ResumePrompt sessions={sessions} onResume={onResume} onFresh={onFresh} onDismiss={onDismiss} />,
    );
    expect(screen.getByText(longPrompt.slice(0, 80) + "...")).toBeInTheDocument();
  });

  it("shows full prompt if under 80 chars (no ellipsis)", () => {
    const shortPrompt = "Fix the login bug";
    const sessions = [makeSession({ last_prompt: shortPrompt })];
    render(
      <ResumePrompt sessions={sessions} onResume={onResume} onFresh={onFresh} onDismiss={onDismiss} />,
    );
    expect(screen.getByText(shortPrompt)).toBeInTheDocument();
    // Should not have an ellipsis
    expect(screen.queryByText(shortPrompt + "...")).not.toBeInTheDocument();
  });

  it("shows working directory", () => {
    const sessions = [makeSession({ working_dir: "/projects/my-app" })];
    render(
      <ResumePrompt sessions={sessions} onResume={onResume} onFresh={onFresh} onDismiss={onDismiss} />,
    );
    expect(screen.getByText("/projects/my-app")).toBeInTheDocument();
  });

  it("Resume button calls onResume with correct taskId", () => {
    const sessions = [makeSession({ task_id: 42 })];
    render(
      <ResumePrompt sessions={sessions} onResume={onResume} onFresh={onFresh} onDismiss={onDismiss} />,
    );
    fireEvent.click(screen.getByTestId("resume-btn"));
    expect(onResume).toHaveBeenCalledWith(42);
  });

  it("Fresh button calls onFresh with correct taskId", () => {
    const sessions = [makeSession({ task_id: 7 })];
    render(
      <ResumePrompt sessions={sessions} onResume={onResume} onFresh={onFresh} onDismiss={onDismiss} />,
    );
    fireEvent.click(screen.getByTestId("fresh-btn"));
    expect(onFresh).toHaveBeenCalledWith(7);
  });

  it("Dismiss button calls onDismiss with correct taskId", () => {
    const sessions = [makeSession({ task_id: 99 })];
    render(
      <ResumePrompt sessions={sessions} onResume={onResume} onFresh={onFresh} onDismiss={onDismiss} />,
    );
    fireEvent.click(screen.getByTestId("dismiss-btn"));
    expect(onDismiss).toHaveBeenCalledWith(99);
  });
});
