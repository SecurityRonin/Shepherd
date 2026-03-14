import { describe, it, expect } from "vitest";
import { render, screen } from "@testing-library/react";

describe("EcosystemManager", () => {
  it("renders all 13 known plugin cards", async () => {
    const { EcosystemManager } = await import("../EcosystemManager");
    render(<EcosystemManager />);
    // Check several plugin names are present
    expect(screen.getByText("Claude Code")).toBeInTheDocument();
    expect(screen.getByText("OpenAI Codex")).toBeInTheDocument();
    expect(screen.getByText("Aider")).toBeInTheDocument();
    expect(screen.getByText("Serena MCP")).toBeInTheDocument();
    expect(screen.getByText("Gamma MCP")).toBeInTheDocument();
    // Should have 13 plugin cards
    const plugins = screen.getAllByText(/MCP|Code|Codex|Aider|Context|OpenCode|Playwright|Firecrawl|Pinecone|Vision|STT|Gamma/);
    expect(plugins.length).toBeGreaterThanOrEqual(13);
  });

  it("renders plugin descriptions", async () => {
    const { EcosystemManager } = await import("../EcosystemManager");
    render(<EcosystemManager />);
    expect(screen.getByText("AI pair programmer")).toBeInTheDocument();
    expect(screen.getByText("Browser automation MCP")).toBeInTheDocument();
  });
});

describe("PluginCard", () => {
  it("shows green 'Detected' badge when plugin is detected", async () => {
    const { PluginCard } = await import("../PluginCard");
    render(<PluginCard plugin={{ id: "test", name: "Test Plugin", description: "desc", detected: true }} />);
    expect(screen.getByTestId("badge-detected-test")).toBeInTheDocument();
    expect(screen.getByText("Detected")).toBeInTheDocument();
  });

  it("shows 'Install' link when not detected and installUrl provided", async () => {
    const { PluginCard } = await import("../PluginCard");
    render(<PluginCard plugin={{ id: "aider", name: "Aider", description: "desc", detected: false, installUrl: "https://aider.chat" }} />);
    expect(screen.getByTestId("install-link-aider")).toBeInTheDocument();
    expect(screen.getByTestId("install-link-aider")).toHaveAttribute("href", "https://aider.chat");
  });

  it("shows neither badge nor install link when not detected and no installUrl", async () => {
    const { PluginCard } = await import("../PluginCard");
    render(<PluginCard plugin={{ id: "test", name: "Test", description: "desc", detected: false }} />);
    expect(screen.queryByTestId("badge-detected-test")).not.toBeInTheDocument();
    expect(screen.queryByTestId("install-link-test")).not.toBeInTheDocument();
  });
});
