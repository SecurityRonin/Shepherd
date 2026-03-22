import { useState, useEffect, useCallback } from 'react';
import { checkTriggers, dismissTrigger } from '../../lib/api';
import type { TriggerSuggestion } from '../../lib/api';

interface TriggerToastProps {
  projectDir: string;
  onNavigate: (route: string) => void;
}

export function TriggerToast({ projectDir, onNavigate }: TriggerToastProps) {
  const [suggestions, setSuggestions] = useState<TriggerSuggestion[]>([]);
  const [dismissed, setDismissed] = useState<Set<string>>(new Set());
  const [visible, setVisible] = useState<string | null>(null);

  useEffect(() => {
    const poll = async () => {
      try {
        const data = await checkTriggers(projectDir);
        setSuggestions(data.filter(s => !dismissed.has(s.id)));
      } catch {
        // Triggers are non-critical — fail silently
      }
    };

    poll();
    const interval = setInterval(poll, 30000);
    return () => clearInterval(interval);
  }, [projectDir, dismissed]);

  useEffect(() => {
    const active = suggestions.find(s => !dismissed.has(s.id));
    if (active && visible !== active.id) {
      const timer = setTimeout(() => setVisible(active.id), 2000);
      return () => clearTimeout(timer);
    }
  }, [suggestions, dismissed, visible]);

  const dismiss = useCallback((id: string) => {
    setDismissed(prev => new Set([...prev, id]));
    setVisible(null);
    dismissTrigger(id, projectDir).catch((err) =>
      console.error("Failed to dismiss trigger:", err),
    );
  }, [projectDir]);

  const activeSuggestion = suggestions.find(s => s.id === visible);
  if (!activeSuggestion) return null;

  return (
    <div className="fixed bottom-6 right-6 z-50 animate-slide-up">
      <div className="bg-white rounded-xl shadow-lg border border-gray-200 p-4 max-w-sm">
        <div className="flex items-start gap-3">
          <div className="w-8 h-8 rounded-lg bg-blue-100 flex items-center justify-center flex-shrink-0">
            <ToolIcon tool={activeSuggestion.tool} />
          </div>
          <div className="flex-1 min-w-0">
            <p className="text-sm text-gray-900 font-medium">{activeSuggestion.message}</p>
            <div className="flex items-center gap-2 mt-2">
              <button
                onClick={() => {
                  onNavigate(activeSuggestion.action_route);
                  dismiss(activeSuggestion.id);
                }}
                className="px-3 py-1 rounded-lg bg-blue-600 text-white text-xs font-medium hover:bg-blue-700"
              >
                {activeSuggestion.action_label}
              </button>
              <button
                onClick={() => dismiss(activeSuggestion.id)}
                className="px-3 py-1 rounded-lg bg-gray-100 text-gray-600 text-xs font-medium hover:bg-gray-200"
              >
                Dismiss
              </button>
            </div>
          </div>
          <button
            onClick={() => dismiss(activeSuggestion.id)}
            className="text-gray-400 hover:text-gray-600"
            aria-label="Close"
          >
            x
          </button>
        </div>
      </div>
    </div>
  );
}

function ToolIcon({ tool }: { tool: string }) {
  const icons: Record<string, string> = {
    name_generator: 'Aa',
    logo_generator: 'Lg',
    north_star: 'NS',
  };
  return (
    <span className="text-blue-600 text-xs font-bold">{icons[tool] || '?'}</span>
  );
}
