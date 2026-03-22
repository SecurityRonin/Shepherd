import { describe, it, expect, beforeEach, vi } from "vitest";
import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";

const mockCreatePr = vi.fn();

vi.mock("../../../lib/api", () => ({
  createPr: (...args: unknown[]) => mockCreatePr(...args),
}));

beforeEach(() => {
  mockCreatePr.mockReset();
});

describe("PrPipeline", () => {
  it("renders branch and task title", async () => {
    const { PrPipeline } = await import("../PrPipeline");
    render(<PrPipeline taskId={1} taskTitle="Add login" branch="feat/login" />);
    expect(screen.getByTestId("pr-pipeline")).toBeInTheDocument();
    expect(screen.getByText("feat/login")).toBeInTheDocument();
    expect(screen.getByText("Add login")).toBeInTheDocument();
  });

  it("renders base branch input defaulting to 'main'", async () => {
    const { PrPipeline } = await import("../PrPipeline");
    render(<PrPipeline taskId={1} taskTitle="Test" branch="feat/test" />);
    const input = screen.getByTestId("base-branch-input") as HTMLInputElement;
    expect(input.value).toBe("main");
  });

  it("renders 'Create PR' button", async () => {
    const { PrPipeline } = await import("../PrPipeline");
    render(<PrPipeline taskId={1} taskTitle="Test" branch="feat/test" />);
    expect(screen.getByTestId("create-pr-btn")).toHaveTextContent("Create PR");
  });

  it("allows changing base branch", async () => {
    const user = userEvent.setup();
    const { PrPipeline } = await import("../PrPipeline");
    render(<PrPipeline taskId={1} taskTitle="Test" branch="feat/test" />);
    const input = screen.getByTestId("base-branch-input") as HTMLInputElement;
    await user.clear(input);
    await user.type(input, "develop");
    expect(input.value).toBe("develop");
  });

  it("calls createPr with correct params when Create PR clicked", async () => {
    const user = userEvent.setup();
    mockCreatePr.mockResolvedValue({ success: true, steps: [], pr_url: null });
    const { PrPipeline } = await import("../PrPipeline");
    render(<PrPipeline taskId={42} taskTitle="Test" branch="feat/test" />);
    await user.click(screen.getByTestId("create-pr-btn"));
    await waitFor(() => {
      expect(mockCreatePr).toHaveBeenCalledWith(42, {
        base_branch: "main",
        auto_commit_message: true,
        run_gates: true,
      });
    });
  });

  it("shows 'Creating...' while running", async () => {
    const user = userEvent.setup();
    mockCreatePr.mockReturnValue(new Promise(() => {}));
    const { PrPipeline } = await import("../PrPipeline");
    render(<PrPipeline taskId={1} taskTitle="Test" branch="feat/test" />);
    await user.click(screen.getByTestId("create-pr-btn"));
    expect(screen.getByTestId("create-pr-btn")).toHaveTextContent("Creating...");
    expect(screen.getByTestId("create-pr-btn")).toBeDisabled();
  });

  it("shows PR URL on success", async () => {
    const user = userEvent.setup();
    mockCreatePr.mockResolvedValue({
      success: true,
      steps: [],
      pr_url: "https://github.com/org/repo/pull/123",
    });
    const { PrPipeline } = await import("../PrPipeline");
    render(<PrPipeline taskId={1} taskTitle="Test" branch="feat/test" />);
    await user.click(screen.getByTestId("create-pr-btn"));
    await waitFor(() => expect(screen.getByTestId("pr-success")).toBeInTheDocument());
    const link = screen.getByText("https://github.com/org/repo/pull/123");
    expect(link.closest("a")).toHaveAttribute("href", "https://github.com/org/repo/pull/123");
  });

  it("shows error message on API failure", async () => {
    const user = userEvent.setup();
    mockCreatePr.mockRejectedValue(new Error("Branch not found"));
    const { PrPipeline } = await import("../PrPipeline");
    render(<PrPipeline taskId={1} taskTitle="Test" branch="feat/test" />);
    await user.click(screen.getByTestId("create-pr-btn"));
    await waitFor(() => {
      expect(screen.getByTestId("pr-error")).toBeInTheDocument();
      expect(screen.getByTestId("pr-error")).toHaveTextContent("Branch not found");
    });
  });

  it("shows pipeline steps when returned", async () => {
    const user = userEvent.setup();
    mockCreatePr.mockResolvedValue({
      success: true,
      steps: [
        { name: "Lint", status: "passed", output: "All clean" },
        { name: "Unit Tests", status: "failed", output: "2 failures" },
      ],
      pr_url: null,
    });
    const { PrPipeline } = await import("../PrPipeline");
    render(<PrPipeline taskId={1} taskTitle="My Feature" branch="feat/test" />);
    await user.click(screen.getByTestId("create-pr-btn"));
    await waitFor(() => expect(screen.getByTestId("pr-steps")).toBeInTheDocument());
    expect(screen.getByText("Lint")).toBeInTheDocument();
    expect(screen.getByText("Unit Tests")).toBeInTheDocument();
  });

  it("shows error when pipeline has failures", async () => {
    const user = userEvent.setup();
    mockCreatePr.mockResolvedValue({
      success: false,
      steps: [{ name: "Lint", status: "failed", output: "Errors" }],
      pr_url: null,
    });
    const { PrPipeline } = await import("../PrPipeline");
    render(<PrPipeline taskId={1} taskTitle="Test" branch="feat/test" />);
    await user.click(screen.getByTestId("create-pr-btn"));
    await waitFor(() => {
      expect(screen.getByTestId("pr-error")).toHaveTextContent("PR pipeline completed with failures.");
    });
  });
});
