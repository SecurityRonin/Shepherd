import { describe, it, expect, beforeEach, vi } from "vitest";
import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { fuzzyMatch } from "../CommandPalette";
import { useStore } from "../../../store";

// Mock the API module
vi.mock("../../../lib/api", () => ({
  approveTask: vi.fn(),
  createTask: vi.fn(),
}));

// --- fuzzyMatch unit tests ---

describe("fuzzyMatch", () => {
  it("exact substring scores highest", () => {
    const result = fuzzyMatch("new", "New Task");
    expect(result.match).toBe(true);
    expect(result.score).toBeGreaterThan(50);
  });

  it("empty query matches everything with score 0", () => {
    const result = fuzzyMatch("", "New Task");
    expect(result.match).toBe(true);
    expect(result.score).toBe(0);
  });

  it("non-matching returns false", () => {
    const result = fuzzyMatch("xyz", "New Task");
    expect(result.match).toBe(false);
  });

  it("fuzzy character match works", () => {
    const result = fuzzyMatch("nt", "New Task");
    expect(result.match).toBe(true);
  });
});

// --- CommandPalette component tests ---

describe("CommandPalette", () => {
  beforeEach(() => {
    useStore.setState({
      tasks: {},
      isCommandPaletteOpen: false,
      isNewTaskDialogOpen: false,
    });
  });

  it("renders search input when open", async () => {
    const { CommandPalette } = await import("../CommandPalette");
    render(<CommandPalette isOpen={true} onClose={() => {}} />);
    expect(screen.getByPlaceholderText(/search/i)).toBeInTheDocument();
  });

  it('shows "No matching commands" for impossible query', async () => {
    const { CommandPalette } = await import("../CommandPalette");
    render(<CommandPalette isOpen={true} onClose={() => {}} />);
    const input = screen.getByPlaceholderText(/search/i);
    await userEvent.type(input, "zzzzzzz");
    expect(screen.getByText(/no matching/i)).toBeInTheDocument();
  });

  it("search input has accessible aria-label", async () => {
    const { CommandPalette } = await import("../CommandPalette");
    render(<CommandPalette isOpen={true} onClose={() => {}} />);
    expect(screen.getByRole("textbox", { name: /search commands/i })).toBeInTheDocument();
  });

  it("does not render when closed", async () => {
    const { CommandPalette } = await import("../CommandPalette");
    const { container } = render(
      <CommandPalette isOpen={false} onClose={() => {}} />,
    );
    expect(container.innerHTML).toBe("");
  });
});
