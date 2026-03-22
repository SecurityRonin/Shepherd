import { describe, it, expect, beforeEach, vi } from "vitest";
import { render, screen, fireEvent, waitFor } from "@testing-library/react";

const mockGenerateLogo = vi.fn();
const mockExportLogo = vi.fn();

vi.mock("../../../lib/api", () => ({
  generateLogo: (...args: unknown[]) => mockGenerateLogo(...args),
  exportLogo: (...args: unknown[]) => mockExportLogo(...args),
}));

beforeEach(() => {
  mockGenerateLogo.mockReset();
  mockExportLogo.mockReset();
});

describe("LogoGenerator", () => {
  // ── Rendering ─────────────────────────────────────────────────

  it("renders the heading and input fields", async () => {
    const { LogoGenerator } = await import("../LogoGenerator");
    render(<LogoGenerator />);
    expect(screen.getByText("Logo Generator")).toBeInTheDocument();
    expect(screen.getByPlaceholderText("Enter product name...")).toBeInTheDocument();
    expect(screen.getByPlaceholderText("Describe your product...")).toBeInTheDocument();
  });

  it("renders all four style options", async () => {
    const { LogoGenerator } = await import("../LogoGenerator");
    render(<LogoGenerator />);
    expect(screen.getByText("Minimal")).toBeInTheDocument();
    expect(screen.getByText("Geometric")).toBeInTheDocument();
    expect(screen.getByText("Mascot")).toBeInTheDocument();
    expect(screen.getByText("Abstract")).toBeInTheDocument();
  });

  it("renders style descriptions", async () => {
    const { LogoGenerator } = await import("../LogoGenerator");
    render(<LogoGenerator />);
    expect(screen.getByText("Clean, flat, modern")).toBeInTheDocument();
    expect(screen.getByText("Precise shapes, symmetry")).toBeInTheDocument();
    expect(screen.getByText("Friendly character, personality")).toBeInTheDocument();
    expect(screen.getByText("Creative, fluid forms")).toBeInTheDocument();
  });

  it("renders two color pickers with default values", async () => {
    const { LogoGenerator } = await import("../LogoGenerator");
    render(<LogoGenerator />);
    expect(screen.getByText("#3B82F6")).toBeInTheDocument();
    expect(screen.getByText("#1E293B")).toBeInTheDocument();
  });

  it("renders generate button disabled when product name is empty", async () => {
    const { LogoGenerator } = await import("../LogoGenerator");
    render(<LogoGenerator />);
    const button = screen.getByText("Generate Logo");
    expect(button).toBeDisabled();
  });

  it("enables generate button when product name is entered", async () => {
    const { LogoGenerator } = await import("../LogoGenerator");
    render(<LogoGenerator />);
    fireEvent.change(screen.getByPlaceholderText("Enter product name..."), {
      target: { value: "MyApp" },
    });
    expect(screen.getByText("Generate Logo")).not.toBeDisabled();
  });

  // ── Style selection ───────────────────────────────────────────

  it("allows selecting a different style", async () => {
    const { LogoGenerator } = await import("../LogoGenerator");
    render(<LogoGenerator />);
    const mascotButton = screen.getByText("Mascot").closest("button")!;
    fireEvent.click(mascotButton);
    expect(mascotButton.className).toContain("border-blue-500");
  });

  // ── API call ──────────────────────────────────────────────────

  it("shows loading state during generation", async () => {
    mockGenerateLogo.mockReturnValue(new Promise(() => {}));
    const { LogoGenerator } = await import("../LogoGenerator");
    render(<LogoGenerator />);
    fireEvent.change(screen.getByPlaceholderText("Enter product name..."), {
      target: { value: "MyApp" },
    });
    fireEvent.click(screen.getByText("Generate Logo"));
    expect(screen.getByText("Generating...")).toBeInTheDocument();
  });

  it("calls generateLogo with correct payload on submit", async () => {
    mockGenerateLogo.mockResolvedValue({ variants: [] });
    const { LogoGenerator } = await import("../LogoGenerator");
    render(<LogoGenerator />);
    fireEvent.change(screen.getByPlaceholderText("Enter product name..."), {
      target: { value: "MyApp" },
    });
    fireEvent.change(screen.getByPlaceholderText("Describe your product..."), {
      target: { value: "A great app" },
    });
    fireEvent.click(screen.getByText("Generate Logo"));
    await waitFor(() => expect(mockGenerateLogo).toHaveBeenCalledTimes(1));
    expect(mockGenerateLogo).toHaveBeenCalledWith({
      product_name: "MyApp",
      product_description: "A great app",
      style: "minimal",
      colors: ["#3B82F6", "#1E293B"],
    });
  });

  it("does not call API when product name is blank", async () => {
    const { LogoGenerator } = await import("../LogoGenerator");
    render(<LogoGenerator />);
    fireEvent.change(screen.getByPlaceholderText("Enter product name..."), {
      target: { value: "   " },
    });
    expect(screen.getByText("Generate Logo")).toBeDisabled();
  });

  // ── Results ───────────────────────────────────────────────────

  it("renders variant images after successful generation", async () => {
    mockGenerateLogo.mockResolvedValue({
      variants: [
        { index: 0, image_data: "base64data0", is_url: false },
        { index: 1, image_data: "https://example.com/logo.png", is_url: true },
      ],
    });
    const { LogoGenerator } = await import("../LogoGenerator");
    render(<LogoGenerator />);
    fireEvent.change(screen.getByPlaceholderText("Enter product name..."), {
      target: { value: "MyApp" },
    });
    fireEvent.click(screen.getByText("Generate Logo"));
    await waitFor(() => expect(screen.getByText("Select a Variant")).toBeInTheDocument());
    expect(screen.getByText("Variant 1")).toBeInTheDocument();
    expect(screen.getByText("Variant 2")).toBeInTheDocument();
    const images = screen.getAllByRole("img");
    expect(images[0]).toHaveAttribute("src", "data:image/png;base64,base64data0");
    expect(images[1]).toHaveAttribute("src", "https://example.com/logo.png");
  });

  it("shows Export Icons button after selecting a variant", async () => {
    mockGenerateLogo.mockResolvedValue({
      variants: [{ index: 0, image_data: "abc", is_url: false }],
    });
    const { LogoGenerator } = await import("../LogoGenerator");
    render(<LogoGenerator />);
    fireEvent.change(screen.getByPlaceholderText("Enter product name..."), {
      target: { value: "MyApp" },
    });
    fireEvent.click(screen.getByText("Generate Logo"));
    await waitFor(() => expect(screen.getByText("Variant 1")).toBeInTheDocument());
    fireEvent.click(screen.getByText("Variant 1").closest("button")!);
    expect(screen.getByText("Export Icons")).toBeInTheDocument();
  });

  // ── Error handling ────────────────────────────────────────────

  it("shows error message on API failure", async () => {
    mockGenerateLogo.mockRejectedValue(new Error("GPU overloaded"));
    const { LogoGenerator } = await import("../LogoGenerator");
    render(<LogoGenerator />);
    fireEvent.change(screen.getByPlaceholderText("Enter product name..."), {
      target: { value: "MyApp" },
    });
    fireEvent.click(screen.getByText("Generate Logo"));
    await waitFor(() => expect(screen.getByText("GPU overloaded")).toBeInTheDocument());
  });

  it("shows fallback error for non-Error rejection", async () => {
    mockGenerateLogo.mockRejectedValue("unknown");
    const { LogoGenerator } = await import("../LogoGenerator");
    render(<LogoGenerator />);
    fireEvent.change(screen.getByPlaceholderText("Enter product name..."), {
      target: { value: "MyApp" },
    });
    fireEvent.click(screen.getByText("Generate Logo"));
    await waitFor(() => expect(screen.getByText("Generation failed")).toBeInTheDocument());
  });

  it("shows error on network failure", async () => {
    mockGenerateLogo.mockRejectedValue(new Error("Network error"));
    const { LogoGenerator } = await import("../LogoGenerator");
    render(<LogoGenerator />);
    fireEvent.change(screen.getByPlaceholderText("Enter product name..."), {
      target: { value: "MyApp" },
    });
    fireEvent.click(screen.getByText("Generate Logo"));
    await waitFor(() => expect(screen.getByText("Network error")).toBeInTheDocument());
  });

  // ── Export ────────────────────────────────────────────────────

  it("calls export API and shows exported files", async () => {
    mockGenerateLogo.mockResolvedValue({
      variants: [{ index: 0, image_data: "abc123", is_url: false }],
    });
    mockExportLogo.mockResolvedValue({
      files: [
        { path: "icons/icon-512.png", format: "png", size_bytes: 2048, dimensions: [512, 512] },
        { path: "icons/icon.ico", format: "ico", size_bytes: 1024, dimensions: null },
      ],
    });
    const { LogoGenerator } = await import("../LogoGenerator");
    render(<LogoGenerator />);
    fireEvent.change(screen.getByPlaceholderText("Enter product name..."), {
      target: { value: "MyApp" },
    });
    fireEvent.click(screen.getByText("Generate Logo"));
    await waitFor(() => expect(screen.getByText("Variant 1")).toBeInTheDocument());
    fireEvent.click(screen.getByText("Variant 1").closest("button")!);
    fireEvent.click(screen.getByText("Export Icons"));
    await waitFor(() => expect(screen.getByText("Export Complete")).toBeInTheDocument());
    expect(screen.getByText("icons/icon-512.png")).toBeInTheDocument();
    expect(screen.getByText("icons/icon.ico")).toBeInTheDocument();
  });

  it("shows export error on failure", async () => {
    mockGenerateLogo.mockResolvedValue({
      variants: [{ index: 0, image_data: "abc123", is_url: false }],
    });
    mockExportLogo.mockRejectedValue(new Error("Export failed"));
    const { LogoGenerator } = await import("../LogoGenerator");
    render(<LogoGenerator />);
    fireEvent.change(screen.getByPlaceholderText("Enter product name..."), {
      target: { value: "MyApp" },
    });
    fireEvent.click(screen.getByText("Generate Logo"));
    await waitFor(() => expect(screen.getByText("Variant 1")).toBeInTheDocument());
    fireEvent.click(screen.getByText("Variant 1").closest("button")!);
    fireEvent.click(screen.getByText("Export Icons"));
    await waitFor(() => expect(screen.getByText("Export failed")).toBeInTheDocument());
  });
});
