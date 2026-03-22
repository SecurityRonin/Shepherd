import React, { useEffect, useState } from "react";
import { PluginCard, type Plugin } from "./PluginCard";
import { getDetectedPlugins } from "../../lib/api";

const KNOWN_PLUGINS: Plugin[] = [
  { id: "claude-code", name: "Claude Code", description: "Anthropic's CLI coding agent", detected: false },
  { id: "codex", name: "OpenAI Codex", description: "OpenAI coding agent", detected: false, installUrl: "https://openai.com/codex" },
  { id: "opencode", name: "OpenCode", description: "Open-source coding agent", detected: false, installUrl: "https://opencode.ai" },
  { id: "aider", name: "Aider", description: "AI pair programmer", detected: false, installUrl: "https://aider.chat" },
  { id: "mistral-vibe", name: "Mistral Vibe", description: "Mistral's CLI coding agent", detected: false, installUrl: "https://github.com/mistralai/mistral-vibe" },
  { id: "serena", name: "Serena MCP", description: "Semantic code navigation MCP", detected: false },
  { id: "context7", name: "Context7 MCP", description: "Library docs MCP server", detected: false },
  { id: "context-mode", name: "Context Mode", description: "Claude Code context management", detected: false },
  { id: "playwright", name: "Playwright MCP", description: "Browser automation MCP", detected: false },
  { id: "firecrawl", name: "Firecrawl MCP", description: "Web scraping MCP server", detected: false },
  { id: "pinecone", name: "Pinecone MCP", description: "Vector search MCP server", detected: false },
  { id: "gemini-vision", name: "Gemini Vision MCP", description: "Image analysis MCP server", detected: false },
  { id: "stt", name: "Local STT MCP", description: "Speech to text MCP server", detected: false },
  { id: "gamma", name: "Gamma MCP", description: "Presentation generation MCP", detected: false },
];

export const EcosystemManager: React.FC = () => {
  const [plugins, setPlugins] = useState<Plugin[]>(KNOWN_PLUGINS);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    getDetectedPlugins()
      .then(({ detected }) => {
        setError(null);
        const detectedSet = new Set(detected);
        setPlugins(
          KNOWN_PLUGINS.map((p) => ({
            ...p,
            detected: detectedSet.has(p.id),
          })),
        );
      })
      .catch((err) => setError(err instanceof Error ? err.message : "Failed to detect plugins"));
  }, []);

  if (error) {
    return (
      <div className="p-6 text-center text-red-400" data-testid="error-message">
        {error}
      </div>
    );
  }

  return (
    <div className="p-6">
      <h2 className="text-xl font-semibold text-white mb-4">Ecosystem Plugins</h2>
      <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 gap-4">
        {plugins.map((plugin) => (
          <PluginCard key={plugin.id} plugin={plugin} />
        ))}
      </div>
    </div>
  );
};
