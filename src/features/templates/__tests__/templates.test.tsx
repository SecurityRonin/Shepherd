import { describe, it, expect, beforeEach, vi } from "vitest";
import { render, screen, waitFor, fireEvent } from "@testing-library/react";
import type { AgentTemplate } from "../../../types";

vi.mock("../../../lib/api", () => ({
  getTemplates: vi.fn(),
}));

import { getTemplates } from "../../../lib/api";

const SAMPLE_TEMPLATES: AgentTemplate[] = [
  {
    id: "tdd-pipeline",
    name: "TDD Pipeline",
    description: "Three-agent TDD workflow",
    category: "pipeline",
    agents: [
      { role: "planner", agent_type: "claude-code", config: { focus: "test-first" } },
      { role: "coder", agent_type: "claude-code", config: {} },
    ],
    quality_gates: ["lint", "test"],
    is_premium: false,
  },
  {
    id: "pair-review",
    name: "Pair Review",
    description: "Two-agent pair programming with review",
    category: "pair",
    agents: [{ role: "reviewer", agent_type: "claude-code", config: {} }],
    quality_gates: ["lint"],
    is_premium: false,
  },
  {
    id: "pro-workflow",
    name: "Pro Workflow",
    description: "Premium multi-agent workflow",
    category: "workflow",
    agents: [
      { role: "architect", agent_type: "claude-code", config: {} },
      { role: "coder", agent_type: "claude-code", config: {} },
      { role: "reviewer", agent_type: "claude-code", config: {} },
    ],
    quality_gates: ["lint", "test", "typecheck"],
    is_premium: true,
  },
];

beforeEach(() => {
  vi.restoreAllMocks();
});

describe("TemplateGallery", () => {
  it("renders loading state initially", async () => {
    // Never resolve so we stay in loading state
    vi.mocked(getTemplates).mockReturnValue(new Promise(() => {}));
    const { TemplateGallery } = await import("../TemplateGallery");
    render(<TemplateGallery />);
    expect(screen.getByTestId("templates-loading")).toBeInTheDocument();
  });

  it("renders template cards after fetch", async () => {
    vi.mocked(getTemplates).mockResolvedValue({ templates: SAMPLE_TEMPLATES });
    const { TemplateGallery } = await import("../TemplateGallery");
    render(<TemplateGallery />);
    await waitFor(() => expect(screen.getByTestId("template-card-tdd-pipeline")).toBeInTheDocument());
    expect(screen.getByTestId("template-card-pair-review")).toBeInTheDocument();
    expect(screen.getByTestId("template-card-pro-workflow")).toBeInTheDocument();
    expect(screen.getByText("TDD Pipeline")).toBeInTheDocument();
    expect(screen.getByText("Three-agent TDD workflow")).toBeInTheDocument();
  });

  it("shows category badges on template cards", async () => {
    vi.mocked(getTemplates).mockResolvedValue({ templates: SAMPLE_TEMPLATES });
    const { TemplateGallery } = await import("../TemplateGallery");
    render(<TemplateGallery />);
    await waitFor(() => expect(screen.getByTestId("template-card-tdd-pipeline")).toBeInTheDocument());
    expect(screen.getByTestId("category-badge-tdd-pipeline")).toHaveTextContent("pipeline");
  });

  it("shows agent count on template cards", async () => {
    vi.mocked(getTemplates).mockResolvedValue({ templates: SAMPLE_TEMPLATES });
    const { TemplateGallery } = await import("../TemplateGallery");
    render(<TemplateGallery />);
    await waitFor(() => expect(screen.getByTestId("agent-count-tdd-pipeline")).toBeInTheDocument());
    expect(screen.getByTestId("agent-count-tdd-pipeline")).toHaveTextContent("2 agents");
    expect(screen.getByTestId("agent-count-pair-review")).toHaveTextContent("1 agent");
  });

  it("shows quality gates on template cards", async () => {
    vi.mocked(getTemplates).mockResolvedValue({ templates: SAMPLE_TEMPLATES });
    const { TemplateGallery } = await import("../TemplateGallery");
    render(<TemplateGallery />);
    await waitFor(() => expect(screen.getByTestId("gates-tdd-pipeline")).toBeInTheDocument());
    expect(screen.getByTestId("gates-tdd-pipeline")).toHaveTextContent("lint");
    expect(screen.getByTestId("gates-tdd-pipeline")).toHaveTextContent("test");
  });

  it("shows premium badge on premium templates", async () => {
    vi.mocked(getTemplates).mockResolvedValue({ templates: SAMPLE_TEMPLATES });
    const { TemplateGallery } = await import("../TemplateGallery");
    render(<TemplateGallery />);
    await waitFor(() => expect(screen.getByTestId("template-card-pro-workflow")).toBeInTheDocument());
    expect(screen.getByTestId("premium-badge-pro-workflow")).toBeInTheDocument();
    // Non-premium templates should NOT have premium badge
    expect(screen.queryByTestId("premium-badge-tdd-pipeline")).not.toBeInTheDocument();
  });

  it("category filter works - shows only matching templates", async () => {
    vi.mocked(getTemplates).mockResolvedValue({ templates: SAMPLE_TEMPLATES });
    const { TemplateGallery } = await import("../TemplateGallery");
    render(<TemplateGallery />);
    await waitFor(() => expect(screen.getByTestId("template-card-tdd-pipeline")).toBeInTheDocument());

    // Click "Pipeline" filter
    fireEvent.click(screen.getByTestId("filter-pipeline"));
    expect(screen.getByTestId("template-card-tdd-pipeline")).toBeInTheDocument();
    expect(screen.queryByTestId("template-card-pair-review")).not.toBeInTheDocument();
    expect(screen.queryByTestId("template-card-pro-workflow")).not.toBeInTheDocument();

    // Click "All" to reset
    fireEvent.click(screen.getByTestId("filter-all"));
    expect(screen.getByTestId("template-card-tdd-pipeline")).toBeInTheDocument();
    expect(screen.getByTestId("template-card-pair-review")).toBeInTheDocument();
    expect(screen.getByTestId("template-card-pro-workflow")).toBeInTheDocument();
  });

  it("shows empty state when no templates", async () => {
    vi.mocked(getTemplates).mockResolvedValue({ templates: [] });
    const { TemplateGallery } = await import("../TemplateGallery");
    render(<TemplateGallery />);
    await waitFor(() => expect(screen.getByTestId("templates-empty")).toBeInTheDocument());
    expect(screen.getByText("No templates available")).toBeInTheDocument();
  });

  it("shows error state when fetch fails", async () => {
    vi.mocked(getTemplates).mockRejectedValue(new Error("Network error"));
    const { TemplateGallery } = await import("../TemplateGallery");
    render(<TemplateGallery />);
    await waitFor(() => expect(screen.getByTestId("templates-error")).toBeInTheDocument());
  });

  it("clicking a template card shows detail view", async () => {
    vi.mocked(getTemplates).mockResolvedValue({ templates: SAMPLE_TEMPLATES });
    const { TemplateGallery } = await import("../TemplateGallery");
    render(<TemplateGallery />);
    await waitFor(() => expect(screen.getByTestId("template-card-tdd-pipeline")).toBeInTheDocument());

    fireEvent.click(screen.getByTestId("template-card-tdd-pipeline"));
    await waitFor(() => expect(screen.getByTestId("template-detail-tdd-pipeline")).toBeInTheDocument());
    expect(screen.getByText("planner")).toBeInTheDocument();
  });
});
