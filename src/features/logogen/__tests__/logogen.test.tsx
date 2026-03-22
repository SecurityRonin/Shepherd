import { describe, it, expect, beforeEach, vi } from "vitest";
import { render, screen, fireEvent, waitFor } from "@testing-library/react";

// Mock global fetch
const mockFetch = vi.fn();
vi.stubGlobal("fetch", mockFetch);

beforeEach(() => {
  vi.restoreAllMocks();
  mockFetch.mockReset();
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
    const input = screen.getByPlaceholderText("Enter product name...");
    fireEvent.change(input, { target: { value: "MyApp" } });
    const button = screen.getByText("Generate Logo");
    expect(button).not.toBeDisabled();
  });

  // ── Style selection ───────────────────────────────────────────

  it("allows selecting a different style", async () => {
    const { LogoGenerator } = await import("../LogoGenerator");
    render(<LogoGenerator />);
    const mascotButton = screen.getByText("Mascot").closest("button")!;
    fireEvent.click(mascotButton);
    // Mascot should now have the selected border style
    expect(mascotButton.className).toContain("border-blue-500");
  });

  // ── API call ──────────────────────────────────────────────────

  it("shows loading state during generation", async () => {
    // Never resolve so we stay in loading state
    mockFetch.mockReturnValue(new Promise(() => {}));
    const { LogoGenerator } = await import("../LogoGenerator");
    render(<LogoGenerator />);
    fireEvent.change(screen.getByPlaceholderText("Enter product name..."), {
      target: { value: "MyApp" },
    });
    fireEvent.click(screen.getByText("Generate Logo"));
    expect(screen.getByText("Generating...")).toBeInTheDocument();
  });

  it("calls /api/logogen with correct payload on submit", async () => {
    mockFetch.mockResolvedValue({
      ok: true,
      json: async () => ({ variants: [] }),
    });
    const { LogoGenerator } = await import("../LogoGenerator");
    render(<LogoGenerator />);
    fireEvent.change(screen.getByPlaceholderText("Enter product name..."), {
      target: { value: "MyApp" },
    });
    fireEvent.change(screen.getByPlaceholderText("Describe your product..."), {
      target: { value: "A great app" },
    });
    fireEvent.click(screen.getByText("Generate Logo"));
    await waitFor(() => expect(mockFetch).toHaveBeenCalledTimes(1));
    const [url, options] = mockFetch.mock.calls[0];
    expect(url).toBe("/api/logogen");
    expect(options.method).toBe("POST");
    const body = JSON.parse(options.body);
    expect(body.product_name).toBe("MyApp");
    expect(body.product_description).toBe("A great app");
    expect(body.style).toBe("minimal");
    expect(body.colors).toEqual(["#3B82F6", "#1E293B"]);
  });

  it("does not call API when product name is blank", async () => {
    const { LogoGenerator } = await import("../LogoGenerator");
    render(<LogoGenerator />);
    // Type spaces only
    fireEvent.change(screen.getByPlaceholderText("Enter product name..."), {
      target: { value: "   " },
    });
    const button = screen.getByText("Generate Logo");
    expect(button).toBeDisabled();
  });

  // ── Results ───────────────────────────────────────────────────

  it("renders variant images after successful generation", async () => {
    mockFetch.mockResolvedValue({
      ok: true,
      json: async () => ({
        variants: [
          { index: 0, image_data: "base64data0", is_url: false },
          { index: 1, image_data: "https://example.com/logo.png", is_url: true },
        ],
      }),
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
    // Check base64 vs URL rendering
    const images = screen.getAllByRole("img");
    expect(images[0]).toHaveAttribute("src", "data:image/png;base64,base64data0");
    expect(images[1]).toHaveAttribute("src", "https://example.com/logo.png");
  });

  it("shows Export Icons button after selecting a variant", async () => {
    mockFetch.mockResolvedValue({
      ok: true,
      json: async () => ({
        variants: [{ index: 0, image_data: "abc", is_url: false }],
      }),
    });
    const { LogoGenerator } = await import("../LogoGenerator");
    render(<LogoGenerator />);
    fireEvent.change(screen.getByPlaceholderText("Enter product name..."), {
      target: { value: "MyApp" },
    });
    fireEvent.click(screen.getByText("Generate Logo"));
    await waitFor(() => expect(screen.getByText("Variant 1")).toBeInTheDocument());
    // Click variant
    fireEvent.click(screen.getByText("Variant 1").closest("button")!);
    expect(screen.getByText("Export Icons")).toBeInTheDocument();
  });

  // ── Error handling ────────────────────────────────────────────

  it("shows error message on API failure", async () => {
    mockFetch.mockResolvedValue({
      ok: false,
      status: 500,
      json: async () => ({ error: "GPU overloaded" }),
    });
    const { LogoGenerator } = await import("../LogoGenerator");
    render(<LogoGenerator />);
    fireEvent.change(screen.getByPlaceholderText("Enter product name..."), {
      target: { value: "MyApp" },
    });
    fireEvent.click(screen.getByText("Generate Logo"));
    await waitFor(() => expect(screen.getByText("GPU overloaded")).toBeInTheDocument());
  });

  it("shows fallback error when response has no error field", async () => {
    mockFetch.mockResolvedValue({
      ok: false,
      status: 502,
      json: async () => { throw new Error("not json"); },
    });
    const { LogoGenerator } = await import("../LogoGenerator");
    render(<LogoGenerator />);
    fireEvent.change(screen.getByPlaceholderText("Enter product name..."), {
      target: { value: "MyApp" },
    });
    fireEvent.click(screen.getByText("Generate Logo"));
    await waitFor(() => expect(screen.getByText("HTTP 502")).toBeInTheDocument());
  });

  it("shows error on network failure", async () => {
    mockFetch.mockRejectedValue(new Error("Network error"));
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
    // First call: generate
    mockFetch.mockResolvedValueOnce({
      ok: true,
      json: async () => ({
        variants: [{ index: 0, image_data: "abc123", is_url: false }],
      }),
    });
    // Second call: export
    mockFetch.mockResolvedValueOnce({
      ok: true,
      json: async () => ({
        files: [
          { path: "icons/icon-512.png", format: "png", size_bytes: 2048, dimensions: [512, 512] },
          { path: "icons/icon.ico", format: "ico", size_bytes: 1024, dimensions: null },
        ],
      }),
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
    mockFetch.mockResolvedValueOnce({
      ok: true,
      json: async () => ({
        variants: [{ index: 0, image_data: "abc123", is_url: false }],
      }),
    });
    mockFetch.mockResolvedValueOnce({
      ok: false,
      status: 500,
      json: async () => ({ error: "Export failed" }),
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
    await waitFor(() => expect(screen.getByText("Export failed")).toBeInTheDocument());
  });
});
