import React from "react";
import { Header } from "./Header";
import { useStore } from "../../store";
import type { ViewMode } from "../../store/ui";

const NAV_ITEMS: Array<{ mode: ViewMode; label: string; icon: string }> = [
  { mode: "overview", label: "Board", icon: "⊞" },
  { mode: "observability", label: "Costs", icon: "💰" },
  { mode: "replay", label: "Replay", icon: "▶" },
  { mode: "ecosystem", label: "Plugins", icon: "🧩" },
  { mode: "cloud", label: "Cloud", icon: "☁" },
];

interface LayoutProps {
  children: React.ReactNode;
}

export const Layout: React.FC<LayoutProps> = ({ children }) => {
  const viewMode = useStore((s) => s.viewMode);
  const setViewMode = useStore((s) => s.setViewMode);

  return (
    <div className="h-screen w-screen flex flex-col bg-shepherd-bg overflow-hidden">
      <Header />
      <div className="flex flex-1 overflow-hidden">
        <nav className="w-14 flex flex-col items-center py-4 gap-4 bg-gray-900 border-r border-gray-700" data-testid="sidebar-nav">
          {NAV_ITEMS.map(({ mode, label, icon }) => (
            <button
              key={mode}
              onClick={() => {
                // Don't switch away from focus mode if in focus
                if (viewMode !== "focus" || mode === "overview") {
                  setViewMode(mode);
                }
              }}
              title={label}
              data-testid={`nav-${mode}`}
              className={`w-10 h-10 flex items-center justify-center rounded text-lg transition-colors ${
                viewMode === mode
                  ? "bg-blue-600 text-white"
                  : "text-gray-400 hover:bg-gray-700 hover:text-white"
              }`}
            >
              {icon}
            </button>
          ))}
        </nav>
        <main className="flex-1 overflow-hidden">{children}</main>
      </div>
    </div>
  );
};
