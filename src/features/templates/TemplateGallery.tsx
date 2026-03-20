import React, { useEffect, useState } from "react";
import { getTemplates } from "../../lib/api";
import type { AgentTemplate, TemplateCategory } from "../../types";

const CATEGORY_FILTERS: Array<{ label: string; value: TemplateCategory | "all" }> = [
  { label: "All", value: "all" },
  { label: "Workflow", value: "workflow" },
  { label: "Pipeline", value: "pipeline" },
  { label: "Pair", value: "pair" },
];

const CATEGORY_COLORS: Record<TemplateCategory, string> = {
  workflow: "bg-blue-500/20 text-blue-400",
  pipeline: "bg-purple-500/20 text-purple-400",
  pair: "bg-green-500/20 text-green-400",
};

interface TemplateCardProps {
  template: AgentTemplate;
  onClick: () => void;
}

const TemplateCard: React.FC<TemplateCardProps> = ({ template, onClick }) => {
  const agentCount = template.agents.length;
  return (
    <div
      data-testid={`template-card-${template.id}`}
      onClick={onClick}
      className="bg-gray-800 rounded-lg p-4 border border-gray-700 hover:border-gray-500 cursor-pointer transition-colors"
    >
      <div className="flex items-start justify-between mb-2">
        <h3 className="text-white font-medium text-sm">{template.name}</h3>
        <div className="flex items-center gap-2">
          {template.is_premium && (
            <span
              data-testid={`premium-badge-${template.id}`}
              className="px-2 py-0.5 text-xs rounded-full bg-yellow-500/20 text-yellow-400 font-medium"
            >
              Premium
            </span>
          )}
          <span
            data-testid={`category-badge-${template.id}`}
            className={`px-2 py-0.5 text-xs rounded-full font-medium ${CATEGORY_COLORS[template.category]}`}
          >
            {template.category}
          </span>
        </div>
      </div>
      <p className="text-gray-400 text-xs mb-3">{template.description}</p>
      <div className="flex items-center justify-between text-xs text-gray-500">
        <span data-testid={`agent-count-${template.id}`}>
          {agentCount} {agentCount === 1 ? "agent" : "agents"}
        </span>
        <span data-testid={`gates-${template.id}`} className="flex items-center gap-1">
          {template.quality_gates.map((gate) => (
            <span key={gate} className="px-1.5 py-0.5 rounded bg-gray-700 text-gray-400">
              {gate}
            </span>
          ))}
        </span>
      </div>
    </div>
  );
};

interface TemplateDetailProps {
  template: AgentTemplate;
  onBack: () => void;
}

const TemplateDetail: React.FC<TemplateDetailProps> = ({ template, onBack }) => {
  return (
    <div data-testid={`template-detail-${template.id}`} className="bg-gray-800 rounded-lg p-6 border border-gray-700">
      <button
        onClick={onBack}
        className="text-gray-400 hover:text-white text-sm mb-4 flex items-center gap-1 transition-colors"
      >
        <span>&larr;</span> Back to templates
      </button>
      <div className="flex items-center gap-3 mb-4">
        <h2 className="text-white text-lg font-semibold">{template.name}</h2>
        <span className={`px-2 py-0.5 text-xs rounded-full font-medium ${CATEGORY_COLORS[template.category]}`}>
          {template.category}
        </span>
        {template.is_premium && (
          <span className="px-2 py-0.5 text-xs rounded-full bg-yellow-500/20 text-yellow-400 font-medium">
            Premium
          </span>
        )}
      </div>
      <p className="text-gray-400 text-sm mb-6">{template.description}</p>

      <div className="mb-6">
        <h3 className="text-white text-sm font-medium mb-2">Agents ({template.agents.length})</h3>
        <div className="space-y-2">
          {template.agents.map((agent, i) => (
            <div key={i} className="bg-gray-900 rounded p-3 flex items-center justify-between">
              <span className="text-white text-sm">{agent.role}</span>
              <span className="text-gray-500 text-xs">{agent.agent_type}</span>
            </div>
          ))}
        </div>
      </div>

      <div>
        <h3 className="text-white text-sm font-medium mb-2">Quality Gates</h3>
        <div className="flex items-center gap-2">
          {template.quality_gates.map((gate) => (
            <span key={gate} className="px-2 py-1 rounded bg-gray-700 text-gray-300 text-xs">
              {gate}
            </span>
          ))}
        </div>
      </div>
    </div>
  );
};

export const TemplateGallery: React.FC = () => {
  const [templates, setTemplates] = useState<AgentTemplate[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [activeFilter, setActiveFilter] = useState<TemplateCategory | "all">("all");
  const [selectedTemplate, setSelectedTemplate] = useState<AgentTemplate | null>(null);

  useEffect(() => {
    let cancelled = false;
    getTemplates()
      .then((res) => {
        if (!cancelled) {
          setTemplates(res.templates);
          setLoading(false);
        }
      })
      .catch((err) => {
        if (!cancelled) {
          setError(err instanceof Error ? err.message : "Failed to load templates");
          setLoading(false);
        }
      });
    return () => { cancelled = true; };
  }, []);

  if (loading) {
    return (
      <div data-testid="templates-loading" className="p-6 flex items-center justify-center">
        <div className="text-gray-400 text-sm">Loading templates...</div>
      </div>
    );
  }

  if (error) {
    return (
      <div data-testid="templates-error" className="p-6 flex items-center justify-center">
        <div className="text-red-400 text-sm">Failed to load templates: {error}</div>
      </div>
    );
  }

  if (selectedTemplate) {
    return (
      <div className="p-6">
        <TemplateDetail template={selectedTemplate} onBack={() => setSelectedTemplate(null)} />
      </div>
    );
  }

  const filteredTemplates = activeFilter === "all"
    ? templates
    : templates.filter((t) => t.category === activeFilter);

  return (
    <div className="p-6">
      <h2 className="text-xl font-semibold text-white mb-4">Template Gallery</h2>

      {/* Filter bar */}
      <div data-testid="template-filters" className="flex items-center gap-2 mb-6">
        {CATEGORY_FILTERS.map((filter) => (
          <button
            key={filter.value}
            data-testid={`filter-${filter.value}`}
            onClick={() => setActiveFilter(filter.value)}
            className={`px-3 py-1.5 text-xs font-medium rounded transition-colors ${
              activeFilter === filter.value
                ? "bg-shepherd-accent text-white"
                : "bg-gray-800 text-gray-400 hover:text-white"
            }`}
          >
            {filter.label}
          </button>
        ))}
      </div>

      {/* Template grid or empty state */}
      {filteredTemplates.length === 0 ? (
        <div data-testid="templates-empty" className="text-center py-12">
          <p className="text-gray-400 text-sm">No templates available</p>
        </div>
      ) : (
        <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 gap-4">
          {filteredTemplates.map((template) => (
            <TemplateCard
              key={template.id}
              template={template}
              onClick={() => setSelectedTemplate(template)}
            />
          ))}
        </div>
      )}
    </div>
  );
};
