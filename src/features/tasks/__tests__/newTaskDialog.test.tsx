import { describe, it, expect, beforeEach, vi } from "vitest";
import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { useStore } from "../../../store";

// Mock the API module
vi.mock("../../../lib/api", () => ({
  createTask: vi.fn(),
}));

describe("NewTaskDialog", () => {
  beforeEach(() => {
    useStore.setState({
      isNewTaskDialogOpen: false,
    });
  });

  it("renders form fields when open", async () => {
    const { NewTaskDialog } = await import("../NewTaskDialog");
    render(<NewTaskDialog isOpen={true} onClose={() => {}} />);
    expect(screen.getByText(/task prompt/i)).toBeInTheDocument();
    expect(screen.getByText(/agent/i)).toBeInTheDocument();
    expect(screen.getByText(/isolation/i)).toBeInTheDocument();
  });

  it("shows validation error for empty prompt on submit", async () => {
    const { NewTaskDialog } = await import("../NewTaskDialog");
    render(<NewTaskDialog isOpen={true} onClose={() => {}} />);
    const submitButton = screen.getByRole("button", { name: /create task/i });
    await userEvent.click(submitButton);
    expect(screen.getByText(/required/i)).toBeInTheDocument();
  });

  it("does not render when closed", async () => {
    const { NewTaskDialog } = await import("../NewTaskDialog");
    const { container } = render(
      <NewTaskDialog isOpen={false} onClose={() => {}} />,
    );
    expect(container.innerHTML).toBe("");
  });
});
