import { describe, it, expect, beforeEach, vi } from "vitest";
import { render, screen, fireEvent, waitFor } from "@testing-library/react";

const mockGenerateNames = vi.fn();

vi.mock("../../../lib/api", () => ({
  generateNames: (...args: unknown[]) => mockGenerateNames(...args),
}));

beforeEach(() => {
  mockGenerateNames.mockReset();
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
  it("renders heading and input fields", async () => {
    const { NameGenerator } = await import("../NameGenerator");
    render(<NameGenerator />);
    expect(screen.getByText("Name Generator")).toBeInTheDocument();
    expect(screen.getByLabelText("Project Description")).toBeInTheDocument();
  });

  it("renders all vibe options", async () => {
    const { NameGenerator } = await import("../NameGenerator");
    render(<NameGenerator />);
    for (const vibe of ["modern", "playful", "enterprise", "minimal", "bold", "friendly", "technical", "abstract", "nature", "futuristic"]) {
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
    fireEvent.change(screen.getByLabelText("Project Description"), { target: { value: "A developer tool" } });
    expect(screen.getByText("Generate Names")).not.toBeDisabled();
  });

  it("toggles vibe selection on click", async () => {
    const { NameGenerator } = await import("../NameGenerator");
    render(<NameGenerator />);
    const modernBtn = screen.getByText("modern");
    expect(modernBtn.className).toContain("bg-white");
    fireEvent.click(modernBtn);
    expect(modernBtn.className).toContain("bg-blue-600");
    fireEvent.click(modernBtn);
    expect(modernBtn.className).toContain("bg-white");
  });

  it("shows loading state during generation", async () => {
    mockGenerateNames.mockReturnValue(new Promise(() => {}));
    const { NameGenerator } = await import("../NameGenerator");
    render(<NameGenerator />);
    fireEvent.change(screen.getByLabelText("Project Description"), { target: { value: "A developer tool" } });
    fireEvent.click(screen.getByText("Generate Names"));
    expect(screen.getByText("Generating...")).toBeInTheDocument();
  });

  it("calls generateNames with correct payload", async () => {
    mockGenerateNames.mockResolvedValue({ candidates: [] });
    const { NameGenerator } = await import("../NameGenerator");
    render(<NameGenerator />);
    fireEvent.change(screen.getByLabelText("Project Description"), { target: { value: "A developer tool" } });
    fireEvent.click(screen.getByText("modern"));
    fireEvent.click(screen.getByText("Generate Names"));
    await waitFor(() => expect(mockGenerateNames).toHaveBeenCalledTimes(1));
    expect(mockGenerateNames).toHaveBeenCalledWith({
      description: "A developer tool",
      vibes: ["modern"],
      count: 20,
    });
  });

  it("renders candidate table after successful generation", async () => {
    mockGenerateNames.mockResolvedValue({ candidates: SAMPLE_CANDIDATES });
    const { NameGenerator } = await import("../NameGenerator");
    render(<NameGenerator />);
    fireEvent.change(screen.getByLabelText("Project Description"), { target: { value: "A developer tool" } });
    fireEvent.click(screen.getByText("Generate Names"));
    await waitFor(() => expect(screen.getByText("Acme")).toBeInTheDocument());
    expect(screen.getByText("Nexus")).toBeInTheDocument();
    expect(screen.getByText("Zephyr")).toBeInTheDocument();
  });

  it("shows domain availability indicators", async () => {
    mockGenerateNames.mockResolvedValue({ candidates: SAMPLE_CANDIDATES });
    const { NameGenerator } = await import("../NameGenerator");
    render(<NameGenerator />);
    fireEvent.change(screen.getByLabelText("Project Description"), { target: { value: "A developer tool" } });
    fireEvent.click(screen.getByText("Generate Names"));
    await waitFor(() => expect(screen.getByText("Acme")).toBeInTheDocument());
    const comDomains = screen.getAllByText(".com");
    expect(comDomains.length).toBe(2);
    expect(screen.getByText(".io")).toBeInTheDocument();
    expect(screen.getByText(".dev")).toBeInTheDocument();
  });

  it("shows status badges for candidates", async () => {
    mockGenerateNames.mockResolvedValue({ candidates: SAMPLE_CANDIDATES });
    const { NameGenerator } = await import("../NameGenerator");
    render(<NameGenerator />);
    fireEvent.change(screen.getByLabelText("Project Description"), { target: { value: "A developer tool" } });
    fireEvent.click(screen.getByText("Generate Names"));
    await waitFor(() => expect(screen.getByText("All Clear")).toBeInTheDocument());
    expect(screen.getByText("Conflicted")).toBeInTheDocument();
    expect(screen.getByText("Partial")).toBeInTheDocument();
  });

  it("shows tagline when present", async () => {
    mockGenerateNames.mockResolvedValue({ candidates: SAMPLE_CANDIDATES });
    const { NameGenerator } = await import("../NameGenerator");
    render(<NameGenerator />);
    fireEvent.change(screen.getByLabelText("Project Description"), { target: { value: "A developer tool" } });
    fireEvent.click(screen.getByText("Generate Names"));
    await waitFor(() => expect(screen.getByText("Build anything")).toBeInTheDocument());
    expect(screen.getByText("Light and fast")).toBeInTheDocument();
  });

  it("shows negative association warnings", async () => {
    mockGenerateNames.mockResolvedValue({ candidates: SAMPLE_CANDIDATES });
    const { NameGenerator } = await import("../NameGenerator");
    render(<NameGenerator />);
    fireEvent.change(screen.getByLabelText("Project Description"), { target: { value: "A developer tool" } });
    fireEvent.click(screen.getByText("Generate Names"));
    await waitFor(() => expect(screen.getByText("Warnings:")).toBeInTheDocument());
    expect(screen.getByText("Common gaming term; Google device brand")).toBeInTheDocument();
  });

  it("disables Select button for conflicted candidates", async () => {
    mockGenerateNames.mockResolvedValue({ candidates: SAMPLE_CANDIDATES });
    const { NameGenerator } = await import("../NameGenerator");
    render(<NameGenerator />);
    fireEvent.change(screen.getByLabelText("Project Description"), { target: { value: "A developer tool" } });
    fireEvent.click(screen.getByText("Generate Names"));
    await waitFor(() => expect(screen.getByText("Acme")).toBeInTheDocument());
    const selectButtons = screen.getAllByText("Select");
    expect(selectButtons[1]).toBeDisabled();
    expect(selectButtons[0]).not.toBeDisabled();
  });

  it("shows selected name and Apply button after clicking Select", async () => {
    mockGenerateNames.mockResolvedValue({ candidates: SAMPLE_CANDIDATES });
    const { NameGenerator } = await import("../NameGenerator");
    render(<NameGenerator />);
    fireEvent.change(screen.getByLabelText("Project Description"), { target: { value: "A developer tool" } });
    fireEvent.click(screen.getByText("Generate Names"));
    await waitFor(() => expect(screen.getByText("Acme")).toBeInTheDocument());
    fireEvent.click(screen.getAllByText("Select")[0]);
    expect(screen.getByText("Apply to Project")).toBeInTheDocument();
    expect(screen.getByText(/Selected:/)).toBeInTheDocument();
  });

  it("shows error message on API failure", async () => {
    mockGenerateNames.mockRejectedValue(new Error("Internal server error"));
    const { NameGenerator } = await import("../NameGenerator");
    render(<NameGenerator />);
    fireEvent.change(screen.getByLabelText("Project Description"), { target: { value: "A developer tool" } });
    fireEvent.click(screen.getByText("Generate Names"));
    await waitFor(() => expect(screen.getByText("Internal server error")).toBeInTheDocument());
  });

  it("shows fallback error for non-Error rejection", async () => {
    mockGenerateNames.mockRejectedValue("something");
    const { NameGenerator } = await import("../NameGenerator");
    render(<NameGenerator />);
    fireEvent.change(screen.getByLabelText("Project Description"), { target: { value: "A developer tool" } });
    fireEvent.click(screen.getByText("Generate Names"));
    await waitFor(() => expect(screen.getByText("Unknown error")).toBeInTheDocument());
  });

  it("shows error on network failure", async () => {
    mockGenerateNames.mockRejectedValue(new Error("Connection refused"));
    const { NameGenerator } = await import("../NameGenerator");
    render(<NameGenerator />);
    fireEvent.change(screen.getByLabelText("Project Description"), { target: { value: "A developer tool" } });
    fireEvent.click(screen.getByText("Generate Names"));
    await waitFor(() => expect(screen.getByText("Connection refused")).toBeInTheDocument());
  });
});
