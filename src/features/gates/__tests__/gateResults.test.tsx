import { describe, it, expect } from "vitest";
import { render, screen } from "@testing-library/react";
import { GateResults } from "../GateResults";
import type { GateResult } from "../GateResults";

function makeGateResult(overrides: Partial<GateResult> = {}): GateResult {
  return {
    gate_name: "ESLint",
    passed: true,
    output: "All rules passed",
    duration_ms: 1234,
    gate_type: "lint",
    ...overrides,
  };
}

describe("GateResults", () => {
  it("renders loading state", () => {
    render(<GateResults results={[]} loading={true} />);
    expect(screen.getByTestId("gate-loading")).toBeInTheDocument();
    expect(screen.getByText("Running quality gates...")).toBeInTheDocument();
  });

  it("renders nothing when results are empty and not loading", () => {
    const { container } = render(<GateResults results={[]} />);
    expect(container.innerHTML).toBe("");
  });

  it("renders gate results with pass count", () => {
    const results = [
      makeGateResult({ gate_name: "ESLint", passed: true }),
      makeGateResult({ gate_name: "TypeCheck", passed: false, gate_type: "type_check" }),
    ];
    render(<GateResults results={results} />);
    expect(screen.getByTestId("gate-header")).toHaveTextContent("1/2 gates passed");
  });

  it("renders all-passed state with green border", () => {
    const results = [
      makeGateResult({ passed: true }),
      makeGateResult({ gate_name: "Tests", passed: true, gate_type: "test" }),
    ];
    render(<GateResults results={results} />);
    const container = screen.getByTestId("gate-results");
    expect(container.className).toContain("border-green-700");
    expect(container.className).toContain("bg-green-900");
    expect(screen.getByTestId("gate-header")).toHaveTextContent("2/2 gates passed");
  });

  it("renders failure state with red border when any gate fails", () => {
    const results = [
      makeGateResult({ passed: true }),
      makeGateResult({ gate_name: "Security", passed: false, gate_type: "security" }),
    ];
    render(<GateResults results={results} />);
    const container = screen.getByTestId("gate-results");
    expect(container.className).toContain("border-red-700");
    expect(container.className).toContain("bg-red-900");
  });

  it("renders gate name for each result", () => {
    const results = [
      makeGateResult({ gate_name: "ESLint" }),
      makeGateResult({ gate_name: "Prettier", gate_type: "format" }),
      makeGateResult({ gate_name: "Jest Tests", gate_type: "test" }),
    ];
    render(<GateResults results={results} />);
    expect(screen.getByText("ESLint")).toBeInTheDocument();
    expect(screen.getByText("Prettier")).toBeInTheDocument();
    expect(screen.getByText("Jest Tests")).toBeInTheDocument();
  });

  it("renders PASS/FAIL labels for each gate", () => {
    const results = [
      makeGateResult({ passed: true }),
      makeGateResult({ gate_name: "Failing", passed: false }),
    ];
    render(<GateResults results={results} />);
    expect(screen.getByText("PASS")).toBeInTheDocument();
    expect(screen.getByText("FAIL")).toBeInTheDocument();
  });

  it("renders correct gate type icons", () => {
    const results = [
      makeGateResult({ gate_type: "lint" }),
      makeGateResult({ gate_name: "Format", gate_type: "format" }),
      makeGateResult({ gate_name: "TypeCheck", gate_type: "type_check" }),
      makeGateResult({ gate_name: "Test", gate_type: "test" }),
      makeGateResult({ gate_name: "Security", gate_type: "security" }),
      makeGateResult({ gate_name: "Custom", gate_type: "custom" }),
    ];
    render(<GateResults results={results} />);

    expect(screen.getByTestId("gate-icon-0")).toHaveTextContent("L");
    expect(screen.getByTestId("gate-icon-1")).toHaveTextContent("F");
    expect(screen.getByTestId("gate-icon-2")).toHaveTextContent("T");
    expect(screen.getByTestId("gate-icon-3")).toHaveTextContent("X");
    expect(screen.getByTestId("gate-icon-4")).toHaveTextContent("S");
    expect(screen.getByTestId("gate-icon-5")).toHaveTextContent("C");
  });

  it("renders '?' icon for unknown gate types", () => {
    const results = [makeGateResult({ gate_type: "unknown_type" })];
    render(<GateResults results={results} />);
    expect(screen.getByTestId("gate-icon-0")).toHaveTextContent("?");
  });

  it("shows duration for each gate", () => {
    const results = [makeGateResult({ duration_ms: 567 })];
    render(<GateResults results={results} />);
    expect(screen.getByText("567ms")).toBeInTheDocument();
  });
});
