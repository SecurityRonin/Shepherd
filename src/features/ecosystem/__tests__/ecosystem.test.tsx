import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen, waitFor } from "@testing-library/react";

const mockGetDetectedPlugins = vi.fn().mockResolvedValue({ detected: [] });

vi.mock("../../../lib/api", () => ({
  getDetectedPlugins: (...args: unknown[]) => mockGetDetectedPlugins(...args),
}));

beforeEach(() => {
  mockGetDetectedPlugins.mockReset().mockResolvedValue({ detected: [] });
});

describe("EcosystemManager", () => {
  it("renders all 14 known plugin cards", async () => {
    const { EcosystemManager } = await import("../EcosystemManager");
    render(<EcosystemManager />);
    expect(screen.getByText("Claude Code")).toBeInTheDocument();
    expect(screen.getByText("OpenAI Codex")).toBeInTheDocument();
    expect(screen.getByText("Aider")).toBeInTheDocument();
    expect(screen.getByText("Mistral Vibe")).toBeInTheDocument();
    expect(screen.getByText("Serena MCP")).toBeInTheDocument();
    expect(screen.getByText("Gamma MCP")).toBeInTheDocument();
    const plugins = screen.getAllByText(/MCP|Code|Codex|Aider|Vibe|Context|OpenCode|Playwright|Firecrawl|Pinecone|Vision|STT|Gamma/);
    expect(plugins.length).toBeGreaterThanOrEqual(14);
  });

  it("renders plugin descriptions", async () => {
    const { EcosystemManager } = await import("../EcosystemManager");
    render(<EcosystemManager />);
    expect(screen.getByText("AI pair programmer")).toBeInTheDocument();
    expect(screen.getByText("Browser automation MCP")).toBeInTheDocument();
    expect(screen.getByText("Mistral's CLI coding agent")).toBeInTheDocument();
  });

  it("shows error message when getDetectedPlugins fails", async () => {
    mockGetDetectedPlugins.mockRejectedValue(new Error("Network error"));
    const { EcosystemManager } = await import("../EcosystemManager");
    render(<EcosystemManager />);
    await waitFor(() => {
      expect(screen.getByTestId("error-message")).toBeInTheDocument();
      expect(screen.getByText("Network error")).toBeInTheDocument();
    });
  });

  it("calls getDetectedPlugins on mount", async () => {
    const { EcosystemManager } = await import("../EcosystemManager");
    render(<EcosystemManager />);
    await waitFor(() => {
      expect(mockGetDetectedPlugins).toHaveBeenCalled();
    });
  });

  it("shows Detected badge for plugins returned by the API", async () => {
    mockGetDetectedPlugins.mockResolvedValue({ detected: ["claude-code", "aider"] });
    const { EcosystemManager } = await import("../EcosystemManager");
    render(<EcosystemManager />);
    await waitFor(() => {
      expect(screen.getByTestId("badge-detected-claude-code")).toBeInTheDocument();
      expect(screen.getByTestId("badge-detected-aider")).toBeInTheDocument();
    });
    // Plugins NOT detected should not have the badge
    expect(screen.queryByTestId("badge-detected-codex")).not.toBeInTheDocument();
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
