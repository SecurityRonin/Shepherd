import { describe, it, expect, beforeEach, vi } from "vitest";
import { render, screen, fireEvent, waitFor } from "@testing-library/react";

const mockFetch = vi.fn();
vi.stubGlobal("fetch", mockFetch);

beforeEach(() => {
  vi.restoreAllMocks();
  mockFetch.mockReset();
});

const SAMPLE_CANDIDATES = [
  {
    name: "Acme",
    tagline: "Build anything",
    reasoning: "Memorable and classic",
    status: "all_clear",
    domains: [
      { tld: "com", domain: "acme.com", available: true },
      { tld: "io", domain: "acme.io", available: false },
    ],
    npm_available: true,
    pypi_available: null,
    github_available: true,
    negative_associations: [],
  },
  {
    name: "Nexus",
    tagline: null,
    reasoning: "Connective meaning",
    status: "conflicted",
    domains: [{ tld: "com", domain: "nexus.com", available: false }],
    npm_available: false,
    pypi_available: false,
    github_available: false,
    negative_associations: ["Common gaming term", "Google device brand"],
  },
  {
    name: "Zephyr",
    tagline: "Light and fast",
    reasoning: "Evokes speed",
    status: "partial",
    domains: [{ tld: "dev", domain: "zephyr.dev", available: true }],
    npm_available: true,
    pypi_available: true,
    github_available: null,
    negative_associations: [],
  },
];

describe("NameGenerator", () => {
  // ── Rendering ─────────────────────────────────────────────────

  it("renders heading and input fields", async () => {
    const { NameGenerator } = await import("../NameGenerator");
    render(<NameGenerator />);
    expect(screen.getByText("Name Generator")).toBeInTheDocument();
    expect(screen.getByLabelText("Project Description")).toBeInTheDocument();
    expect(screen.getByPlaceholderText("Describe your project in a few sentences...")).toBeInTheDocument();
  });

  it("renders all vibe options", async () => {
    const { NameGenerator } = await import("../NameGenerator");
    render(<NameGenerator />);
    const vibes = ["modern", "playful", "enterprise", "minimal", "bold", "friendly", "technical", "abstract", "nature", "futuristic"];
    for (const vibe of vibes) {
      expect(screen.getByText(vibe)).toBeInTheDocument();
    }
  });

  it("renders generate button disabled when description is empty", async () => {
    const { NameGenerator } = await import("../NameGenerator");
    render(<NameGenerator />);
    expect(screen.getByText("Generate Names")).toBeDisabled();
  });

  it("enables generate button when description is entered", async () => {
    const { NameGenerator } = await import("../NameGenerator");
    render(<NameGenerator />);
    fireEvent.change(screen.getByLabelText("Project Description"), {
      target: { value: "A developer tool" },
    });
    expect(screen.getByText("Generate Names")).not.toBeDisabled();
  });

  // ── Vibe selection ────────────────────────────────────────────

  it("toggles vibe selection on click", async () => {
    const { NameGenerator } = await import("../NameGenerator");
    render(<NameGenerator />);
    const modernBtn = screen.getByText("modern");
    // Initially unselected
    expect(modernBtn.className).toContain("bg-white");
    // Select
    fireEvent.click(modernBtn);
    expect(modernBtn.className).toContain("bg-blue-600");
    // Deselect
    fireEvent.click(modernBtn);
    expect(modernBtn.className).toContain("bg-white");
  });

  // ── API call ──────────────────────────────────────────────────

  it("shows loading state during generation", async () => {
    mockFetch.mockReturnValue(new Promise(() => {}));
    const { NameGenerator } = await import("../NameGenerator");
    render(<NameGenerator />);
    fireEvent.change(screen.getByLabelText("Project Description"), {
      target: { value: "A developer tool" },
    });
    fireEvent.click(screen.getByText("Generate Names"));
    expect(screen.getByText("Generating...")).toBeInTheDocument();
  });

  it("calls /api/namegen with correct payload", async () => {
    mockFetch.mockResolvedValue({
      ok: true,
      json: async () => ({ candidates: [] }),
    });
    const { NameGenerator } = await import("../NameGenerator");
    render(<NameGenerator />);
    fireEvent.change(screen.getByLabelText("Project Description"), {
      target: { value: "A developer tool" },
    });
    // Select a vibe
    fireEvent.click(screen.getByText("modern"));
    fireEvent.click(screen.getByText("Generate Names"));
    await waitFor(() => expect(mockFetch).toHaveBeenCalledTimes(1));
    const [url, options] = mockFetch.mock.calls[0];
    expect(url).toBe("/api/namegen");
    expect(options.method).toBe("POST");
    const body = JSON.parse(options.body);
    expect(body.description).toBe("A developer tool");
    expect(body.vibes).toEqual(["modern"]);
    expect(body.count).toBe(20);
  });

  // ── Results ───────────────────────────────────────────────────

  it("renders candidate table after successful generation", async () => {
    mockFetch.mockResolvedValue({
      ok: true,
      json: async () => ({ candidates: SAMPLE_CANDIDATES }),
    });
    const { NameGenerator } = await import("../NameGenerator");
    render(<NameGenerator />);
    fireEvent.change(screen.getByLabelText("Project Description"), {
      target: { value: "A developer tool" },
    });
    fireEvent.click(screen.getByText("Generate Names"));
    await waitFor(() => expect(screen.getByText("Acme")).toBeInTheDocument());
    expect(screen.getByText("Nexus")).toBeInTheDocument();
    expect(screen.getByText("Zephyr")).toBeInTheDocument();
  });

  it("shows domain availability indicators", async () => {
    mockFetch.mockResolvedValue({
      ok: true,
      json: async () => ({ candidates: SAMPLE_CANDIDATES }),
    });
    const { NameGenerator } = await import("../NameGenerator");
    render(<NameGenerator />);
    fireEvent.change(screen.getByLabelText("Project Description"), {
      target: { value: "A developer tool" },
    });
    fireEvent.click(screen.getByText("Generate Names"));
    await waitFor(() => expect(screen.getByText("Acme")).toBeInTheDocument());
    // Check domain TLD labels — .com appears for both Acme and Nexus
    const comDomains = screen.getAllByText(".com");
    expect(comDomains.length).toBe(2);
    expect(screen.getByText(".io")).toBeInTheDocument();
    expect(screen.getByText(".dev")).toBeInTheDocument();
  });

  it("shows status badges for candidates", async () => {
    mockFetch.mockResolvedValue({
      ok: true,
      json: async () => ({ candidates: SAMPLE_CANDIDATES }),
    });
    const { NameGenerator } = await import("../NameGenerator");
    render(<NameGenerator />);
    fireEvent.change(screen.getByLabelText("Project Description"), {
      target: { value: "A developer tool" },
    });
    fireEvent.click(screen.getByText("Generate Names"));
    await waitFor(() => expect(screen.getByText("All Clear")).toBeInTheDocument());
    expect(screen.getByText("Conflicted")).toBeInTheDocument();
    expect(screen.getByText("Partial")).toBeInTheDocument();
  });

  it("shows tagline when present", async () => {
    mockFetch.mockResolvedValue({
      ok: true,
      json: async () => ({ candidates: SAMPLE_CANDIDATES }),
    });
    const { NameGenerator } = await import("../NameGenerator");
    render(<NameGenerator />);
    fireEvent.change(screen.getByLabelText("Project Description"), {
      target: { value: "A developer tool" },
    });
    fireEvent.click(screen.getByText("Generate Names"));
    await waitFor(() => expect(screen.getByText("Build anything")).toBeInTheDocument());
    expect(screen.getByText("Light and fast")).toBeInTheDocument();
  });

  it("shows negative association warnings", async () => {
    mockFetch.mockResolvedValue({
      ok: true,
      json: async () => ({ candidates: SAMPLE_CANDIDATES }),
    });
    const { NameGenerator } = await import("../NameGenerator");
    render(<NameGenerator />);
    fireEvent.change(screen.getByLabelText("Project Description"), {
      target: { value: "A developer tool" },
    });
    fireEvent.click(screen.getByText("Generate Names"));
    await waitFor(() => expect(screen.getByText("Warnings:")).toBeInTheDocument());
    expect(screen.getByText("Common gaming term; Google device brand")).toBeInTheDocument();
  });

  it("disables Select button for conflicted candidates", async () => {
    mockFetch.mockResolvedValue({
      ok: true,
      json: async () => ({ candidates: SAMPLE_CANDIDATES }),
    });
    const { NameGenerator } = await import("../NameGenerator");
    render(<NameGenerator />);
    fireEvent.change(screen.getByLabelText("Project Description"), {
      target: { value: "A developer tool" },
    });
    fireEvent.click(screen.getByText("Generate Names"));
    await waitFor(() => expect(screen.getByText("Acme")).toBeInTheDocument());
    const selectButtons = screen.getAllByText("Select");
    // The Nexus row (index 1) should be disabled
    expect(selectButtons[1]).toBeDisabled();
    // Acme (index 0) should be enabled
    expect(selectButtons[0]).not.toBeDisabled();
  });

  it("shows selected name and Apply button after clicking Select", async () => {
    mockFetch.mockResolvedValue({
      ok: true,
      json: async () => ({ candidates: SAMPLE_CANDIDATES }),
    });
    const { NameGenerator } = await import("../NameGenerator");
    render(<NameGenerator />);
    fireEvent.change(screen.getByLabelText("Project Description"), {
      target: { value: "A developer tool" },
    });
    fireEvent.click(screen.getByText("Generate Names"));
    await waitFor(() => expect(screen.getByText("Acme")).toBeInTheDocument());
    const selectButtons = screen.getAllByText("Select");
    fireEvent.click(selectButtons[0]);
    expect(screen.getByText("Apply to Project")).toBeInTheDocument();
    expect(screen.getByText(/Selected:/)).toBeInTheDocument();
  });

  // ── Error handling ────────────────────────────────────────────

  it("shows error message on API failure", async () => {
    mockFetch.mockResolvedValue({
      ok: false,
      status: 500,
      json: async () => ({ error: "Internal server error" }),
    });
    const { NameGenerator } = await import("../NameGenerator");
    render(<NameGenerator />);
    fireEvent.change(screen.getByLabelText("Project Description"), {
      target: { value: "A developer tool" },
    });
    fireEvent.click(screen.getByText("Generate Names"));
    await waitFor(() => expect(screen.getByText("Internal server error")).toBeInTheDocument());
  });

  it("shows fallback error for non-JSON error responses", async () => {
    mockFetch.mockResolvedValue({
      ok: false,
      status: 503,
      json: async () => { throw new Error("not json"); },
    });
    const { NameGenerator } = await import("../NameGenerator");
    render(<NameGenerator />);
    fireEvent.change(screen.getByLabelText("Project Description"), {
      target: { value: "A developer tool" },
    });
    fireEvent.click(screen.getByText("Generate Names"));
    await waitFor(() => expect(screen.getByText("Request failed with status 503")).toBeInTheDocument());
  });

  it("shows error on network failure", async () => {
    mockFetch.mockRejectedValue(new Error("Connection refused"));
    const { NameGenerator } = await import("../NameGenerator");
    render(<NameGenerator />);
    fireEvent.change(screen.getByLabelText("Project Description"), {
      target: { value: "A developer tool" },
    });
    fireEvent.click(screen.getByText("Generate Names"));
    await waitFor(() => expect(screen.getByText("Connection refused")).toBeInTheDocument());
  });
});
