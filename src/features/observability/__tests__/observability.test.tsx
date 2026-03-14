import { describe, it, expect, beforeEach, vi } from "vitest";
import { render, screen } from "@testing-library/react";
import { useStore } from "../../../store";

beforeEach(() => {
  useStore.setState({
    agentMetrics: [],
    spendingSummary: null,
    replayEvents: [],
  } as any);
});

// Mock fetch
vi.stubGlobal("fetch", vi.fn(() => Promise.resolve({ json: () => Promise.resolve(null) })));

describe("CostDashboard", () => {
  it("renders 'No spending data' when summary is null", async () => {
    const { CostDashboard } = await import("../CostDashboard");
    render(<CostDashboard />);
    expect(screen.getByTestId("no-spending")).toBeInTheDocument();
    expect(screen.getByText("No spending data")).toBeInTheDocument();
  });

  it("renders agent rows when summary has data", async () => {
    useStore.setState({
      spendingSummary: {
        total_cost_usd: 1.23,
        total_tokens: 50000,
        total_tasks: 5,
        total_llm_calls: 20,
        by_agent: [{ agent_id: "claude-code", total_cost_usd: 1.23, total_tokens: 50000, task_count: 5 }],
        by_model: [],
      },
    } as any);
    const { CostDashboard } = await import("../CostDashboard");
    render(<CostDashboard />);
    expect(screen.getByText("claude-code")).toBeInTheDocument();
    expect(screen.getByText("$1.2300")).toBeInTheDocument();
  });

  it("renders total tasks and LLM calls", async () => {
    useStore.setState({
      spendingSummary: {
        total_cost_usd: 0.5,
        total_tokens: 10000,
        total_tasks: 3,
        total_llm_calls: 10,
        by_agent: [],
        by_model: [],
      },
    } as any);
    const { CostDashboard } = await import("../CostDashboard");
    render(<CostDashboard />);
    expect(screen.getByText(/3 tasks/)).toBeInTheDocument();
    expect(screen.getByText(/10 LLM calls/)).toBeInTheDocument();
  });
});

describe("BudgetBar", () => {
  it("renders without turning red below 80%", async () => {
    const { BudgetBar } = await import("../BudgetBar");
    render(<BudgetBar used={3.0} limit={5.0} label="Daily Budget" />);
    const fill = screen.getByTestId("budget-bar-fill");
    expect(fill.className).not.toContain("bg-red-500");
    expect(fill.className).toContain("bg-blue-500");
  });

  it("turns red at >=80% usage", async () => {
    const { BudgetBar } = await import("../BudgetBar");
    render(<BudgetBar used={4.0} limit={5.0} />);
    const fill = screen.getByTestId("budget-bar-fill");
    expect(fill.className).toContain("bg-red-500");
  });

  it("renders label when provided", async () => {
    const { BudgetBar } = await import("../BudgetBar");
    render(<BudgetBar used={1.0} limit={5.0} label="My Budget" />);
    expect(screen.getByText("My Budget")).toBeInTheDocument();
  });
});

describe("AgentSpendingRow", () => {
  it("renders agent id and cost", async () => {
    const { AgentSpendingRow } = await import("../AgentSpendingRow");
    const spending = { agent_id: "codex", total_cost_usd: 0.5432, total_tokens: 25000, task_count: 3 };
    render(<table><tbody><AgentSpendingRow spending={spending} /></tbody></table>);
    expect(screen.getByText("codex")).toBeInTheDocument();
    expect(screen.getByText("$0.5432")).toBeInTheDocument();
    expect(screen.getByText("3")).toBeInTheDocument();
  });
});
