import { describe, it, expect, beforeEach, vi } from "vitest";
import { render, screen, fireEvent, waitFor } from "@testing-library/react";

const mockFetch = vi.fn();
vi.stubGlobal("fetch", mockFetch);

const SAMPLE_PHASES = [
  { id: 1, name: "Market Analysis", description: "Analyze target market", document_count: 3 },
  { id: 2, name: "User Research", description: "Define user personas", document_count: 2 },
  { id: 3, name: "Strategy", description: "Define go-to-market strategy", document_count: 4 },
];

beforeEach(() => {
  vi.restoreAllMocks();
  mockFetch.mockReset();
});

describe("NorthStarWizard", () => {
  // ── Rendering ─────────────────────────────────────────────────

  it("renders the heading", async () => {
    // Phases fetch — never resolves so it stays in initial state
    mockFetch.mockReturnValue(new Promise(() => {}));
    const { NorthStarWizard } = await import("../NorthStarWizard");
    render(<NorthStarWizard />);
    expect(screen.getByText("North Star PMF Analysis")).toBeInTheDocument();
  });

  it("renders input fields for product name and description", async () => {
    mockFetch.mockResolvedValue({
      ok: true,
      json: async () => ({ phases: SAMPLE_PHASES, total: 3 }),
    });
    const { NorthStarWizard } = await import("../NorthStarWizard");
    render(<NorthStarWizard />);
    expect(screen.getByPlaceholderText("Enter product name...")).toBeInTheDocument();
    expect(screen.getByPlaceholderText("Describe your product, target market, and key features...")).toBeInTheDocument();
  });

  it("fetches phases on mount and shows Start Analysis button with phase count", async () => {
    mockFetch.mockResolvedValue({
      ok: true,
      json: async () => ({ phases: SAMPLE_PHASES, total: 3 }),
    });
    const { NorthStarWizard } = await import("../NorthStarWizard");
    render(<NorthStarWizard />);
    await waitFor(() => expect(screen.getByText("Start Analysis (3 Phases)")).toBeInTheDocument());
  });

  it("disables start button when inputs are empty", async () => {
    mockFetch.mockResolvedValue({
      ok: true,
      json: async () => ({ phases: SAMPLE_PHASES, total: 3 }),
    });
    const { NorthStarWizard } = await import("../NorthStarWizard");
    render(<NorthStarWizard />);
    await waitFor(() => expect(screen.getByText("Start Analysis (3 Phases)")).toBeInTheDocument());
    expect(screen.getByText("Start Analysis (3 Phases)")).toBeDisabled();
  });

  it("enables start button when both fields are filled", async () => {
    mockFetch.mockResolvedValue({
      ok: true,
      json: async () => ({ phases: SAMPLE_PHASES, total: 3 }),
    });
    const { NorthStarWizard } = await import("../NorthStarWizard");
    render(<NorthStarWizard />);
    await waitFor(() => expect(screen.getByText("Start Analysis (3 Phases)")).toBeInTheDocument());
    fireEvent.change(screen.getByPlaceholderText("Enter product name..."), {
      target: { value: "MyProduct" },
    });
    fireEvent.change(screen.getByPlaceholderText("Describe your product, target market, and key features..."), {
      target: { value: "A great product" },
    });
    expect(screen.getByText("Start Analysis (3 Phases)")).not.toBeDisabled();
  });

  // ── Error on phases load ──────────────────────────────────────

  it("shows error when phase fetch fails", async () => {
    mockFetch.mockResolvedValue({
      ok: false,
      status: 500,
    });
    const { NorthStarWizard } = await import("../NorthStarWizard");
    render(<NorthStarWizard />);
    await waitFor(() => expect(screen.getByText("HTTP 500")).toBeInTheDocument());
  });

  // ── Analysis execution ────────────────────────────────────────

  it("shows Analyzing... and phase progress during analysis", async () => {
    // First call: fetch phases
    mockFetch.mockResolvedValueOnce({
      ok: true,
      json: async () => ({ phases: SAMPLE_PHASES, total: 3 }),
    });
    // Phase 1: never resolves
    mockFetch.mockReturnValueOnce(new Promise(() => {}));

    const { NorthStarWizard } = await import("../NorthStarWizard");
    render(<NorthStarWizard />);
    await waitFor(() => expect(screen.getByText("Start Analysis (3 Phases)")).toBeInTheDocument());

    fireEvent.change(screen.getByPlaceholderText("Enter product name..."), {
      target: { value: "MyProduct" },
    });
    fireEvent.change(screen.getByPlaceholderText("Describe your product, target market, and key features..."), {
      target: { value: "A great product" },
    });
    fireEvent.click(screen.getByText("Start Analysis (3 Phases)"));
    expect(screen.getByText("Analyzing...")).toBeInTheDocument();
    expect(screen.getByText("Phase Progress")).toBeInTheDocument();
  });

  it("shows all phase names during analysis", async () => {
    mockFetch.mockResolvedValueOnce({
      ok: true,
      json: async () => ({ phases: SAMPLE_PHASES, total: 3 }),
    });
    mockFetch.mockReturnValueOnce(new Promise(() => {}));

    const { NorthStarWizard } = await import("../NorthStarWizard");
    render(<NorthStarWizard />);
    await waitFor(() => expect(screen.getByText("Start Analysis (3 Phases)")).toBeInTheDocument());

    fireEvent.change(screen.getByPlaceholderText("Enter product name..."), {
      target: { value: "MyProduct" },
    });
    fireEvent.change(screen.getByPlaceholderText("Describe your product, target market, and key features..."), {
      target: { value: "A great product" },
    });
    fireEvent.click(screen.getByText("Start Analysis (3 Phases)"));

    expect(screen.getByText(/Phase 1: Market Analysis/)).toBeInTheDocument();
    expect(screen.getByText(/Phase 2: User Research/)).toBeInTheDocument();
    expect(screen.getByText(/Phase 3: Strategy/)).toBeInTheDocument();
  });

  it("completes all phases and shows completion summary", async () => {
    // Fetch phases
    mockFetch.mockResolvedValueOnce({
      ok: true,
      json: async () => ({ phases: SAMPLE_PHASES, total: 3 }),
    });
    // Phase 1 result
    mockFetch.mockResolvedValueOnce({
      ok: true,
      json: async () => ({
        phase_id: 1,
        phase_name: "Market Analysis",
        status: "completed",
        output: "Market analysis complete",
        documents: [{ title: "Market Report", filename: "market.md", doc_type: "report" }],
      }),
    });
    // Phase 2 result
    mockFetch.mockResolvedValueOnce({
      ok: true,
      json: async () => ({
        phase_id: 2,
        phase_name: "User Research",
        status: "completed",
        output: "User research complete",
        documents: [],
      }),
    });
    // Phase 3 result
    mockFetch.mockResolvedValueOnce({
      ok: true,
      json: async () => ({
        phase_id: 3,
        phase_name: "Strategy",
        status: "completed",
        output: "Strategy complete",
        documents: [{ title: "Strategy Doc", filename: "strategy.md", doc_type: "doc" }],
      }),
    });

    const { NorthStarWizard } = await import("../NorthStarWizard");
    render(<NorthStarWizard />);
    await waitFor(() => expect(screen.getByText("Start Analysis (3 Phases)")).toBeInTheDocument());

    fireEvent.change(screen.getByPlaceholderText("Enter product name..."), {
      target: { value: "MyProduct" },
    });
    fireEvent.change(screen.getByPlaceholderText("Describe your product, target market, and key features..."), {
      target: { value: "A great product" },
    });
    fireEvent.click(screen.getByText("Start Analysis (3 Phases)"));

    await waitFor(() => expect(screen.getByText("Analysis Complete")).toBeInTheDocument());
    expect(screen.getByText(/3 phases completed successfully/)).toBeInTheDocument();
    expect(screen.getByText(/2 documents generated/)).toBeInTheDocument();
  });

  it("shows generated documents for completed phases", async () => {
    mockFetch.mockResolvedValueOnce({
      ok: true,
      json: async () => ({ phases: [SAMPLE_PHASES[0]], total: 1 }),
    });
    mockFetch.mockResolvedValueOnce({
      ok: true,
      json: async () => ({
        phase_id: 1,
        phase_name: "Market Analysis",
        status: "completed",
        output: "Done",
        documents: [
          { title: "Market Report", filename: "market.md", doc_type: "report" },
          { title: "Competitor Analysis", filename: "competitors.md", doc_type: "report" },
        ],
      }),
    });

    const { NorthStarWizard } = await import("../NorthStarWizard");
    render(<NorthStarWizard />);
    await waitFor(() => expect(screen.getByText("Start Analysis (1 Phases)")).toBeInTheDocument());

    fireEvent.change(screen.getByPlaceholderText("Enter product name..."), {
      target: { value: "MyProduct" },
    });
    fireEvent.change(screen.getByPlaceholderText("Describe your product, target market, and key features..."), {
      target: { value: "A great product" },
    });
    fireEvent.click(screen.getByText("Start Analysis (1 Phases)"));

    await waitFor(() => expect(screen.getByText("Analysis Complete")).toBeInTheDocument());
    expect(screen.getByText(/Market Report \(market\.md\)/)).toBeInTheDocument();
    expect(screen.getByText(/Competitor Analysis \(competitors\.md\)/)).toBeInTheDocument();
  });

  // ── Phase failure ─────────────────────────────────────────────

  it("shows error and stops when a phase fails", async () => {
    mockFetch.mockResolvedValueOnce({
      ok: true,
      json: async () => ({ phases: SAMPLE_PHASES, total: 3 }),
    });
    // Phase 1 fails
    mockFetch.mockResolvedValueOnce({
      ok: false,
      status: 500,
      json: async () => ({ error: "Phase 1 exploded" }),
    });

    const { NorthStarWizard } = await import("../NorthStarWizard");
    render(<NorthStarWizard />);
    await waitFor(() => expect(screen.getByText("Start Analysis (3 Phases)")).toBeInTheDocument());

    fireEvent.change(screen.getByPlaceholderText("Enter product name..."), {
      target: { value: "MyProduct" },
    });
    fireEvent.change(screen.getByPlaceholderText("Describe your product, target market, and key features..."), {
      target: { value: "A great product" },
    });
    fireEvent.click(screen.getByText("Start Analysis (3 Phases)"));

    await waitFor(() => expect(screen.getByText("Phase 1 exploded")).toBeInTheDocument());
    // Analysis should not show complete
    expect(screen.queryByText("Analysis Complete")).not.toBeInTheDocument();
  });

  it("disables inputs during analysis", async () => {
    mockFetch.mockResolvedValueOnce({
      ok: true,
      json: async () => ({ phases: SAMPLE_PHASES, total: 3 }),
    });
    mockFetch.mockReturnValueOnce(new Promise(() => {}));

    const { NorthStarWizard } = await import("../NorthStarWizard");
    render(<NorthStarWizard />);
    await waitFor(() => expect(screen.getByText("Start Analysis (3 Phases)")).toBeInTheDocument());

    fireEvent.change(screen.getByPlaceholderText("Enter product name..."), {
      target: { value: "MyProduct" },
    });
    fireEvent.change(screen.getByPlaceholderText("Describe your product, target market, and key features..."), {
      target: { value: "A great product" },
    });
    fireEvent.click(screen.getByText("Start Analysis (3 Phases)"));

    expect(screen.getByPlaceholderText("Enter product name...")).toBeDisabled();
    expect(screen.getByPlaceholderText("Describe your product, target market, and key features...")).toBeDisabled();
  });

  it("shows progress counter during analysis", async () => {
    mockFetch.mockResolvedValueOnce({
      ok: true,
      json: async () => ({ phases: SAMPLE_PHASES, total: 3 }),
    });
    mockFetch.mockReturnValueOnce(new Promise(() => {}));

    const { NorthStarWizard } = await import("../NorthStarWizard");
    render(<NorthStarWizard />);
    await waitFor(() => expect(screen.getByText("Start Analysis (3 Phases)")).toBeInTheDocument());

    fireEvent.change(screen.getByPlaceholderText("Enter product name..."), {
      target: { value: "MyProduct" },
    });
    fireEvent.change(screen.getByPlaceholderText("Describe your product, target market, and key features..."), {
      target: { value: "A great product" },
    });
    fireEvent.click(screen.getByText("Start Analysis (3 Phases)"));

    expect(screen.getByText(/0\/3 phases/)).toBeInTheDocument();
  });

  it("calls /api/northstar/phase with correct payload", async () => {
    mockFetch.mockResolvedValueOnce({
      ok: true,
      json: async () => ({ phases: [SAMPLE_PHASES[0]], total: 1 }),
    });
    mockFetch.mockResolvedValueOnce({
      ok: true,
      json: async () => ({
        phase_id: 1,
        phase_name: "Market Analysis",
        status: "completed",
        output: "Done",
        documents: [],
      }),
    });

    const { NorthStarWizard } = await import("../NorthStarWizard");
    render(<NorthStarWizard />);
    await waitFor(() => expect(screen.getByText("Start Analysis (1 Phases)")).toBeInTheDocument());

    fireEvent.change(screen.getByPlaceholderText("Enter product name..."), {
      target: { value: "MyProduct" },
    });
    fireEvent.change(screen.getByPlaceholderText("Describe your product, target market, and key features..."), {
      target: { value: "A great product" },
    });
    fireEvent.click(screen.getByText("Start Analysis (1 Phases)"));

    await waitFor(() => expect(mockFetch).toHaveBeenCalledTimes(2));
    const [url, options] = mockFetch.mock.calls[1];
    expect(url).toBe("/api/northstar/phase");
    expect(options.method).toBe("POST");
    const body = JSON.parse(options.body);
    expect(body.product_name).toBe("MyProduct");
    expect(body.product_description).toBe("A great product");
    expect(body.phase_id).toBe(1);
  });
});
