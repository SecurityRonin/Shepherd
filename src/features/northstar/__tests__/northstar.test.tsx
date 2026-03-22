import { describe, it, expect, beforeEach, vi } from "vitest";
import { render, screen, fireEvent, waitFor } from "@testing-library/react";

const mockGetNorthStarPhases = vi.fn();
const mockExecuteNorthStarPhase = vi.fn();

vi.mock("../../../lib/api", () => ({
  getNorthStarPhases: (...args: unknown[]) => mockGetNorthStarPhases(...args),
  executeNorthStarPhase: (...args: unknown[]) => mockExecuteNorthStarPhase(...args),
}));

const SAMPLE_PHASES = [
  { id: 1, name: "Market Analysis", description: "Analyze target market", document_count: 3 },
  { id: 2, name: "User Research", description: "Define user personas", document_count: 2 },
  { id: 3, name: "Strategy", description: "Define go-to-market strategy", document_count: 4 },
];

beforeEach(() => {
  mockGetNorthStarPhases.mockReset();
  mockExecuteNorthStarPhase.mockReset();
});

describe("NorthStarWizard", () => {
  it("renders the heading", async () => {
    mockGetNorthStarPhases.mockReturnValue(new Promise(() => {}));
    const { NorthStarWizard } = await import("../NorthStarWizard");
    render(<NorthStarWizard />);
    expect(screen.getByText("North Star PMF Analysis")).toBeInTheDocument();
  });

  it("renders input fields for product name and description", async () => {
    mockGetNorthStarPhases.mockResolvedValue({ phases: SAMPLE_PHASES, total: 3 });
    const { NorthStarWizard } = await import("../NorthStarWizard");
    render(<NorthStarWizard />);
    expect(screen.getByPlaceholderText("Enter product name...")).toBeInTheDocument();
    expect(screen.getByPlaceholderText("Describe your product, target market, and key features...")).toBeInTheDocument();
  });

  it("fetches phases on mount and shows Start Analysis button with phase count", async () => {
    mockGetNorthStarPhases.mockResolvedValue({ phases: SAMPLE_PHASES, total: 3 });
    const { NorthStarWizard } = await import("../NorthStarWizard");
    render(<NorthStarWizard />);
    await waitFor(() => expect(screen.getByText("Start Analysis (3 Phases)")).toBeInTheDocument());
  });

  it("disables start button when inputs are empty", async () => {
    mockGetNorthStarPhases.mockResolvedValue({ phases: SAMPLE_PHASES, total: 3 });
    const { NorthStarWizard } = await import("../NorthStarWizard");
    render(<NorthStarWizard />);
    await waitFor(() => expect(screen.getByText("Start Analysis (3 Phases)")).toBeInTheDocument());
    expect(screen.getByText("Start Analysis (3 Phases)")).toBeDisabled();
  });

  it("enables start button when both fields are filled", async () => {
    mockGetNorthStarPhases.mockResolvedValue({ phases: SAMPLE_PHASES, total: 3 });
    const { NorthStarWizard } = await import("../NorthStarWizard");
    render(<NorthStarWizard />);
    await waitFor(() => expect(screen.getByText("Start Analysis (3 Phases)")).toBeInTheDocument());
    fireEvent.change(screen.getByPlaceholderText("Enter product name..."), { target: { value: "MyProduct" } });
    fireEvent.change(screen.getByPlaceholderText("Describe your product, target market, and key features..."), { target: { value: "A great product" } });
    expect(screen.getByText("Start Analysis (3 Phases)")).not.toBeDisabled();
  });

  it("shows error when phase fetch fails", async () => {
    mockGetNorthStarPhases.mockRejectedValue(new Error("HTTP 500"));
    const { NorthStarWizard } = await import("../NorthStarWizard");
    render(<NorthStarWizard />);
    await waitFor(() => expect(screen.getByText("HTTP 500")).toBeInTheDocument());
  });

  it("shows Analyzing... and phase progress during analysis", async () => {
    mockGetNorthStarPhases.mockResolvedValue({ phases: SAMPLE_PHASES, total: 3 });
    mockExecuteNorthStarPhase.mockReturnValue(new Promise(() => {}));
    const { NorthStarWizard } = await import("../NorthStarWizard");
    render(<NorthStarWizard />);
    await waitFor(() => expect(screen.getByText("Start Analysis (3 Phases)")).toBeInTheDocument());
    fireEvent.change(screen.getByPlaceholderText("Enter product name..."), { target: { value: "MyProduct" } });
    fireEvent.change(screen.getByPlaceholderText("Describe your product, target market, and key features..."), { target: { value: "A great product" } });
    fireEvent.click(screen.getByText("Start Analysis (3 Phases)"));
    expect(screen.getByText("Analyzing...")).toBeInTheDocument();
    expect(screen.getByText("Phase Progress")).toBeInTheDocument();
  });

  it("shows all phase names during analysis", async () => {
    mockGetNorthStarPhases.mockResolvedValue({ phases: SAMPLE_PHASES, total: 3 });
    mockExecuteNorthStarPhase.mockReturnValue(new Promise(() => {}));
    const { NorthStarWizard } = await import("../NorthStarWizard");
    render(<NorthStarWizard />);
    await waitFor(() => expect(screen.getByText("Start Analysis (3 Phases)")).toBeInTheDocument());
    fireEvent.change(screen.getByPlaceholderText("Enter product name..."), { target: { value: "MyProduct" } });
    fireEvent.change(screen.getByPlaceholderText("Describe your product, target market, and key features..."), { target: { value: "A great product" } });
    fireEvent.click(screen.getByText("Start Analysis (3 Phases)"));
    expect(screen.getByText(/Phase 1: Market Analysis/)).toBeInTheDocument();
    expect(screen.getByText(/Phase 2: User Research/)).toBeInTheDocument();
    expect(screen.getByText(/Phase 3: Strategy/)).toBeInTheDocument();
  });

  it("completes all phases and shows completion summary", async () => {
    mockGetNorthStarPhases.mockResolvedValue({ phases: SAMPLE_PHASES, total: 3 });
    mockExecuteNorthStarPhase
      .mockResolvedValueOnce({ phase_id: 1, phase_name: "Market Analysis", status: "completed", output: "Done", documents: [{ title: "Market Report", filename: "market.md", doc_type: "report" }] })
      .mockResolvedValueOnce({ phase_id: 2, phase_name: "User Research", status: "completed", output: "Done", documents: [] })
      .mockResolvedValueOnce({ phase_id: 3, phase_name: "Strategy", status: "completed", output: "Done", documents: [{ title: "Strategy Doc", filename: "strategy.md", doc_type: "doc" }] });

    const { NorthStarWizard } = await import("../NorthStarWizard");
    render(<NorthStarWizard />);
    await waitFor(() => expect(screen.getByText("Start Analysis (3 Phases)")).toBeInTheDocument());
    fireEvent.change(screen.getByPlaceholderText("Enter product name..."), { target: { value: "MyProduct" } });
    fireEvent.change(screen.getByPlaceholderText("Describe your product, target market, and key features..."), { target: { value: "A great product" } });
    fireEvent.click(screen.getByText("Start Analysis (3 Phases)"));
    await waitFor(() => expect(screen.getByText("Analysis Complete")).toBeInTheDocument());
    expect(screen.getByText(/3 phases completed successfully/)).toBeInTheDocument();
    expect(screen.getByText(/2 documents generated/)).toBeInTheDocument();
  });

  it("shows error and stops when a phase fails", async () => {
    mockGetNorthStarPhases.mockResolvedValue({ phases: SAMPLE_PHASES, total: 3 });
    mockExecuteNorthStarPhase.mockRejectedValue(new Error("Phase 1 exploded"));

    const { NorthStarWizard } = await import("../NorthStarWizard");
    render(<NorthStarWizard />);
    await waitFor(() => expect(screen.getByText("Start Analysis (3 Phases)")).toBeInTheDocument());
    fireEvent.change(screen.getByPlaceholderText("Enter product name..."), { target: { value: "MyProduct" } });
    fireEvent.change(screen.getByPlaceholderText("Describe your product, target market, and key features..."), { target: { value: "A great product" } });
    fireEvent.click(screen.getByText("Start Analysis (3 Phases)"));
    await waitFor(() => expect(screen.getByText("Phase 1 exploded")).toBeInTheDocument());
    expect(screen.queryByText("Analysis Complete")).not.toBeInTheDocument();
  });

  it("disables inputs during analysis", async () => {
    mockGetNorthStarPhases.mockResolvedValue({ phases: SAMPLE_PHASES, total: 3 });
    mockExecuteNorthStarPhase.mockReturnValue(new Promise(() => {}));
    const { NorthStarWizard } = await import("../NorthStarWizard");
    render(<NorthStarWizard />);
    await waitFor(() => expect(screen.getByText("Start Analysis (3 Phases)")).toBeInTheDocument());
    fireEvent.change(screen.getByPlaceholderText("Enter product name..."), { target: { value: "MyProduct" } });
    fireEvent.change(screen.getByPlaceholderText("Describe your product, target market, and key features..."), { target: { value: "A great product" } });
    fireEvent.click(screen.getByText("Start Analysis (3 Phases)"));
    expect(screen.getByPlaceholderText("Enter product name...")).toBeDisabled();
  });

  it("shows progress counter during analysis", async () => {
    mockGetNorthStarPhases.mockResolvedValue({ phases: SAMPLE_PHASES, total: 3 });
    mockExecuteNorthStarPhase.mockReturnValue(new Promise(() => {}));
    const { NorthStarWizard } = await import("../NorthStarWizard");
    render(<NorthStarWizard />);
    await waitFor(() => expect(screen.getByText("Start Analysis (3 Phases)")).toBeInTheDocument());
    fireEvent.change(screen.getByPlaceholderText("Enter product name..."), { target: { value: "MyProduct" } });
    fireEvent.change(screen.getByPlaceholderText("Describe your product, target market, and key features..."), { target: { value: "A great product" } });
    fireEvent.click(screen.getByText("Start Analysis (3 Phases)"));
    expect(screen.getByText(/0\/3 phases/)).toBeInTheDocument();
  });

  it("calls executeNorthStarPhase with correct payload", async () => {
    mockGetNorthStarPhases.mockResolvedValue({ phases: [SAMPLE_PHASES[0]], total: 1 });
    mockExecuteNorthStarPhase.mockResolvedValue({ phase_id: 1, phase_name: "Market Analysis", status: "completed", output: "Done", documents: [] });

    const { NorthStarWizard } = await import("../NorthStarWizard");
    render(<NorthStarWizard />);
    await waitFor(() => expect(screen.getByText("Start Analysis (1 Phases)")).toBeInTheDocument());
    fireEvent.change(screen.getByPlaceholderText("Enter product name..."), { target: { value: "MyProduct" } });
    fireEvent.change(screen.getByPlaceholderText("Describe your product, target market, and key features..."), { target: { value: "A great product" } });
    fireEvent.click(screen.getByText("Start Analysis (1 Phases)"));
    await waitFor(() => expect(mockExecuteNorthStarPhase).toHaveBeenCalledTimes(1));
    expect(mockExecuteNorthStarPhase).toHaveBeenCalledWith({
      product_name: "MyProduct",
      product_description: "A great product",
      phase_id: 1,
      previous_context: undefined,
    });
  });
});
