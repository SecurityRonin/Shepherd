import { describe, it, expect, beforeEach, vi } from "vitest";
import { render, screen, fireEvent } from "@testing-library/react";

beforeEach(() => {
  vi.restoreAllMocks();
});

// ── WizardStepper ───────────────────────────────────────────────

describe("WizardStepper", () => {
  const defaultPhases = [
    { phase: "north_star", label: "North Star", status: "pending" as const },
    { phase: "name_gen", label: "Brand Name", status: "pending" as const },
    { phase: "logo_gen", label: "Logo & Identity", status: "pending" as const },
    { phase: "superpowers", label: "Superpowers", status: "pending" as const },
  ];

  it("renders all phase labels", async () => {
    const { WizardStepper } = await import("../WizardStepper");
    render(<WizardStepper phases={defaultPhases} currentIndex={0} onJump={vi.fn()} />);
    expect(screen.getByText("North Star")).toBeInTheDocument();
    expect(screen.getByText("Brand Name")).toBeInTheDocument();
    expect(screen.getByText("Logo & Identity")).toBeInTheDocument();
    expect(screen.getByText("Superpowers")).toBeInTheDocument();
  });

  it("highlights the current step with blue", async () => {
    const { WizardStepper } = await import("../WizardStepper");
    render(<WizardStepper phases={defaultPhases} currentIndex={1} onJump={vi.fn()} />);
    const buttons = screen.getAllByRole("button");
    // Current index (1) should have blue styling
    expect(buttons[1].className).toContain("bg-blue-600");
    // Others should not
    expect(buttons[0].className).not.toContain("bg-blue-600");
    expect(buttons[2].className).not.toContain("bg-blue-600");
  });

  it("shows step numbers for pending phases", async () => {
    const { WizardStepper } = await import("../WizardStepper");
    render(<WizardStepper phases={defaultPhases} currentIndex={0} onJump={vi.fn()} />);
    const buttons = screen.getAllByRole("button");
    expect(buttons[0]).toHaveTextContent("1");
    expect(buttons[1]).toHaveTextContent("2");
    expect(buttons[2]).toHaveTextContent("3");
    expect(buttons[3]).toHaveTextContent("4");
  });

  it("shows checkmark for completed phases", async () => {
    const phases = [
      { ...defaultPhases[0], status: "completed" as const },
      ...defaultPhases.slice(1),
    ];
    const { WizardStepper } = await import("../WizardStepper");
    render(<WizardStepper phases={phases} currentIndex={1} onJump={vi.fn()} />);
    const buttons = screen.getAllByRole("button");
    expect(buttons[0]).toHaveTextContent("\u2713");
    expect(buttons[0].className).toContain("bg-green-500");
  });

  it("shows dash for skipped phases", async () => {
    const phases = [
      { ...defaultPhases[0], status: "skipped" as const },
      ...defaultPhases.slice(1),
    ];
    const { WizardStepper } = await import("../WizardStepper");
    render(<WizardStepper phases={phases} currentIndex={1} onJump={vi.fn()} />);
    const buttons = screen.getAllByRole("button");
    expect(buttons[0]).toHaveTextContent("\u2014");
    expect(buttons[0].className).toContain("bg-gray-300");
  });

  it("calls onJump with the correct index when clicked", async () => {
    const onJump = vi.fn();
    const { WizardStepper } = await import("../WizardStepper");
    render(<WizardStepper phases={defaultPhases} currentIndex={0} onJump={onJump} />);
    const buttons = screen.getAllByRole("button");
    fireEvent.click(buttons[2]);
    expect(onJump).toHaveBeenCalledWith(2);
  });

  it("shows connecting lines between steps", async () => {
    const { WizardStepper } = await import("../WizardStepper");
    const { container } = render(
      <WizardStepper phases={defaultPhases} currentIndex={0} onJump={vi.fn()} />,
    );
    // There should be 3 connector lines (between 4 steps)
    const connectors = container.querySelectorAll(".h-px");
    expect(connectors.length).toBe(3);
  });

  it("shows green connector line after completed phase", async () => {
    const phases = [
      { ...defaultPhases[0], status: "completed" as const },
      ...defaultPhases.slice(1),
    ];
    const { WizardStepper } = await import("../WizardStepper");
    const { container } = render(
      <WizardStepper phases={phases} currentIndex={1} onJump={vi.fn()} />,
    );
    const connectors = container.querySelectorAll(".h-px");
    expect(connectors[0].className).toContain("bg-green-400");
    expect(connectors[1].className).toContain("bg-gray-200");
  });

  it("highlights current step label text", async () => {
    const { WizardStepper } = await import("../WizardStepper");
    render(<WizardStepper phases={defaultPhases} currentIndex={2} onJump={vi.fn()} />);
    // Current step label should be darker
    const label = screen.getByText("Logo & Identity");
    expect(label.className).toContain("text-gray-900");
    // Non-current should be lighter
    const otherLabel = screen.getByText("North Star");
    expect(otherLabel.className).toContain("text-gray-500");
  });
});

// ── ProjectWizard ───────────────────────────────────────────────

describe("ProjectWizard", () => {
  const defaultProps = {
    projectId: "proj-1",
    onNavigate: vi.fn(),
    onComplete: vi.fn(),
    onDismiss: vi.fn(),
  };

  it("renders the wizard heading and description", async () => {
    const { ProjectWizard } = await import("../ProjectWizard");
    render(<ProjectWizard {...defaultProps} />);
    expect(screen.getByText("New Project Setup")).toBeInTheDocument();
    expect(screen.getByText(/Optional guided journey/)).toBeInTheDocument();
  });

  it("renders all four phases in the stepper", async () => {
    const { ProjectWizard } = await import("../ProjectWizard");
    render(<ProjectWizard {...defaultProps} />);
    // "North Star" appears twice: once in stepper label, once in phase content heading
    expect(screen.getAllByText("North Star").length).toBeGreaterThanOrEqual(2);
    // Others appear in both stepper and possibly content
    expect(screen.getAllByText("Brand Name").length).toBeGreaterThanOrEqual(1);
    expect(screen.getAllByText("Logo & Identity").length).toBeGreaterThanOrEqual(1);
    expect(screen.getAllByText("Superpowers").length).toBeGreaterThanOrEqual(1);
  });

  it("shows the current phase content with Start and Skip buttons", async () => {
    const { ProjectWizard } = await import("../ProjectWizard");
    render(<ProjectWizard {...defaultProps} />);
    // First phase content should be shown (heading + description)
    expect(screen.getAllByText("North Star").length).toBeGreaterThanOrEqual(1);
    expect(screen.getByText("Define your product strategy, target audience, and success metrics")).toBeInTheDocument();
    expect(screen.getByText("Start")).toBeInTheDocument();
    expect(screen.getByText("Skip")).toBeInTheDocument();
  });

  it("Start button navigates to the correct route", async () => {
    const onNavigate = vi.fn();
    const { ProjectWizard } = await import("../ProjectWizard");
    render(<ProjectWizard {...defaultProps} onNavigate={onNavigate} />);
    fireEvent.click(screen.getByText("Start"));
    expect(onNavigate).toHaveBeenCalledWith("/tools/northstar");
  });

  it("Skip button advances to the next phase", async () => {
    const { ProjectWizard } = await import("../ProjectWizard");
    render(<ProjectWizard {...defaultProps} />);
    // Initially on North Star
    expect(screen.getByText("Define your product strategy, target audience, and success metrics")).toBeInTheDocument();
    fireEvent.click(screen.getByText("Skip"));
    // Should advance to Brand Name
    expect(screen.getByText("Brainstorm and validate product names with domain availability")).toBeInTheDocument();
  });

  it("navigates through all phases via Skip", async () => {
    const { ProjectWizard } = await import("../ProjectWizard");
    render(<ProjectWizard {...defaultProps} />);
    // Skip phase 1 (North Star)
    fireEvent.click(screen.getByText("Skip"));
    // Now on Brand Name
    expect(screen.getByText("Brainstorm and validate product names with domain availability")).toBeInTheDocument();
    // Skip phase 2
    fireEvent.click(screen.getByText("Skip"));
    // Now on Logo & Identity
    expect(screen.getByText("Generate app icons and visual identity assets")).toBeInTheDocument();
    // Skip phase 3
    fireEvent.click(screen.getByText("Skip"));
    // Now on Superpowers
    expect(screen.getByText("Install Obra Superpowers for enhanced agent capabilities")).toBeInTheDocument();
  });

  it("calls onComplete when all phases are skipped", async () => {
    const onComplete = vi.fn();
    const { ProjectWizard } = await import("../ProjectWizard");
    render(<ProjectWizard {...defaultProps} onComplete={onComplete} />);
    fireEvent.click(screen.getByText("Skip")); // 1
    fireEvent.click(screen.getByText("Skip")); // 2
    fireEvent.click(screen.getByText("Skip")); // 3
    fireEvent.click(screen.getByText("Skip")); // 4
    expect(onComplete).toHaveBeenCalledTimes(1);
  });

  it("shows completion state when all phases are done", async () => {
    const { ProjectWizard } = await import("../ProjectWizard");
    render(<ProjectWizard {...defaultProps} />);
    fireEvent.click(screen.getByText("Skip")); // 1
    fireEvent.click(screen.getByText("Skip")); // 2
    fireEvent.click(screen.getByText("Skip")); // 3
    fireEvent.click(screen.getByText("Skip")); // 4
    expect(screen.getByText("Project setup complete!")).toBeInTheDocument();
    expect(screen.getByText(/You can always revisit/)).toBeInTheDocument();
  });

  it("dismiss button calls onDismiss", async () => {
    const onDismiss = vi.fn();
    const { ProjectWizard } = await import("../ProjectWizard");
    render(<ProjectWizard {...defaultProps} onDismiss={onDismiss} />);
    fireEvent.click(screen.getByText("Dismiss wizard"));
    expect(onDismiss).toHaveBeenCalledTimes(1);
  });

  it("clicking stepper jumps to selected phase", async () => {
    const { ProjectWizard } = await import("../ProjectWizard");
    render(<ProjectWizard {...defaultProps} />);
    // Click the 3rd step button in the stepper (Logo & Identity)
    const stepButtons = screen.getAllByRole("button").filter(
      (btn) => btn.className.includes("rounded-full"),
    );
    fireEvent.click(stepButtons[2]);
    // Should show Logo & Identity content
    expect(screen.getByText("Generate app icons and visual identity assets")).toBeInTheDocument();
  });

  it("Start button shows Continue when phase is in progress", async () => {
    const { ProjectWizard } = await import("../ProjectWizard");
    render(<ProjectWizard {...defaultProps} />);
    // Click Start to set phase to in_progress
    fireEvent.click(screen.getByText("Start"));
    // Jump back to same phase
    const stepButtons = screen.getAllByRole("button").filter(
      (btn) => btn.className.includes("rounded-full"),
    );
    fireEvent.click(stepButtons[0]);
    expect(screen.getByText("Continue")).toBeInTheDocument();
  });

  it("navigates to correct routes for each phase", async () => {
    const onNavigate = vi.fn();
    const { ProjectWizard } = await import("../ProjectWizard");
    render(<ProjectWizard {...defaultProps} onNavigate={onNavigate} />);

    // Phase 1: North Star
    fireEvent.click(screen.getByText("Start"));
    expect(onNavigate).toHaveBeenLastCalledWith("/tools/northstar");

    // Skip to phase 2 and start
    fireEvent.click(screen.getByText("Skip"));
    fireEvent.click(screen.getByText("Start"));
    expect(onNavigate).toHaveBeenLastCalledWith("/tools/namegen");

    // Skip to phase 3 and start
    fireEvent.click(screen.getByText("Skip"));
    fireEvent.click(screen.getByText("Start"));
    expect(onNavigate).toHaveBeenLastCalledWith("/tools/logogen");

    // Skip to phase 4 and start
    fireEvent.click(screen.getByText("Skip"));
    fireEvent.click(screen.getByText("Start"));
    expect(onNavigate).toHaveBeenLastCalledWith("/settings/superpowers");
  });
});
