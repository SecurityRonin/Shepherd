import React, { useState, useEffect, useRef, useMemo, useCallback } from "react";
import { useStore } from "../../store";
import { approveTask } from "../../lib/api";

// --- Fuzzy match ---

export function fuzzyMatch(
  query: string,
  text: string,
): { match: boolean; score: number } {
  if (query === "") return { match: true, score: 0 };

  const lowerQuery = query.toLowerCase();
  const lowerText = text.toLowerCase();

  // Exact substring match — highest score
  const substringIndex = lowerText.indexOf(lowerQuery);
  if (substringIndex !== -1) {
    return { match: true, score: 100 - substringIndex };
  }

  // Character-by-character fuzzy match
  let score = 0;
  let textIdx = 0;
  let prevMatchIdx = -2; // track consecutive matches

  for (let qi = 0; qi < lowerQuery.length; qi++) {
    const ch = lowerQuery[qi];
    let found = false;
    while (textIdx < lowerText.length) {
      if (lowerText[textIdx] === ch) {
        // Consecutive bonus
        if (textIdx === prevMatchIdx + 1) {
          score += 10;
        }
        // Word boundary bonus (start of text or preceded by space/separator)
        if (
          textIdx === 0 ||
          lowerText[textIdx - 1] === " " ||
          lowerText[textIdx - 1] === "-" ||
          lowerText[textIdx - 1] === "_"
        ) {
          score += 8;
        }
        score += 1;
        prevMatchIdx = textIdx;
        textIdx++;
        found = true;
        break;
      }
      textIdx++;
    }
    if (!found) return { match: false, score: 0 };
  }

  return { match: true, score };
}

// --- Types ---

interface PaletteAction {
  id: string;
  label: string;
  category: "approve" | "task" | "view" | "lifecycle";
  shortcut?: string;
  execute: () => void;
}

interface CommandPaletteProps {
  isOpen: boolean;
  onClose: () => void;
}

// --- Component ---

export const CommandPalette: React.FC<CommandPaletteProps> = ({
  isOpen,
  onClose,
}) => {
  const [query, setQuery] = useState("");
  const [selectedIndex, setSelectedIndex] = useState(0);
  const inputRef = useRef<HTMLInputElement>(null);
  const listRef = useRef<HTMLDivElement>(null);

  const tasks = useStore((s) => s.tasks);
  const enterFocus = useStore((s) => s.enterFocus);
  const exitFocus = useStore((s) => s.exitFocus);
  const toggleView = useStore((s) => s.toggleView);
  const setNewTaskDialogOpen = useStore((s) => s.setNewTaskDialogOpen);

  // Build list of actions
  const actions = useMemo((): PaletteAction[] => {
    const list: PaletteAction[] = [];

    // View actions
    list.push({
      id: "toggle-view",
      label: "Toggle View",
      category: "view",
      shortcut: "\u2318 0",
      execute: () => toggleView(),
    });

    // Task actions
    list.push({
      id: "new-task",
      label: "New Task",
      category: "task",
      shortcut: "\u2318 N",
      execute: () => setNewTaskDialogOpen(true),
    });

    // Approve all
    const inputTasks = Object.values(tasks).filter(
      (t) => t.status === "input",
    );
    list.push({
      id: "approve-all",
      label: "Approve All",
      category: "approve",
      shortcut: "\u2318 \u21e7 \u23ce",
      execute: () => {
        for (const t of inputTasks) {
          approveTask(t.id).catch(console.error);
        }
      },
    });

    // Lifecycle placeholders
    list.push({
      id: "lifecycle-name-gen",
      label: "Name Generator",
      category: "lifecycle",
      execute: () => console.log("Name Generator triggered"),
    });
    list.push({
      id: "lifecycle-logo-gen",
      label: "Logo Generator",
      category: "lifecycle",
      execute: () => console.log("Logo Generator triggered"),
    });
    list.push({
      id: "lifecycle-north-star",
      label: "North Star PMF",
      category: "lifecycle",
      execute: () => console.log("North Star PMF triggered"),
    });

    // Per-task: focus and approve
    const taskList = Object.values(tasks);
    for (const t of taskList) {
      list.push({
        id: `focus-${t.id}`,
        label: `Focus: ${t.title}`,
        category: "task",
        execute: () => enterFocus(t.id),
      });
      if (t.status === "input") {
        list.push({
          id: `approve-${t.id}`,
          label: `Approve: ${t.title}`,
          category: "approve",
          execute: () => {
            approveTask(t.id).catch(console.error);
          },
        });
      }
    }

    return list;
  }, [tasks, enterFocus, exitFocus, toggleView, setNewTaskDialogOpen]);

  // Filter and sort by fuzzy match
  const filtered = useMemo(() => {
    if (!query) return actions;
    return actions
      .map((action) => ({ action, ...fuzzyMatch(query, action.label) }))
      .filter((r) => r.match)
      .sort((a, b) => b.score - a.score)
      .map((r) => r.action);
  }, [query, actions]);

  // Group by category
  const grouped = useMemo(() => {
    const groups: Record<string, PaletteAction[]> = {};
    for (const action of filtered) {
      if (!groups[action.category]) groups[action.category] = [];
      groups[action.category].push(action);
    }
    return groups;
  }, [filtered]);

  const categoryOrder = ["approve", "task", "view", "lifecycle"] as const;
  const categoryLabels: Record<string, string> = {
    approve: "Approve",
    task: "Tasks",
    view: "View",
    lifecycle: "Lifecycle",
  };

  // Reset state when opening
  useEffect(() => {
    if (isOpen) {
      setQuery("");
      setSelectedIndex(0);
      // Focus input on next tick
      setTimeout(() => inputRef.current?.focus(), 0);
    }
  }, [isOpen]);

  // Keyboard navigation
  const handleKeyDown = useCallback(
    (e: React.KeyboardEvent) => {
      if (e.key === "ArrowDown") {
        e.preventDefault();
        setSelectedIndex((prev) => Math.min(prev + 1, filtered.length - 1));
      } else if (e.key === "ArrowUp") {
        e.preventDefault();
        setSelectedIndex((prev) => Math.max(prev - 1, 0));
      } else if (e.key === "Enter") {
        e.preventDefault();
        if (filtered[selectedIndex]) {
          filtered[selectedIndex].execute();
          onClose();
        }
      } else if (e.key === "Escape") {
        e.preventDefault();
        onClose();
      }
    },
    [filtered, selectedIndex, onClose],
  );

  // Scroll selected item into view
  useEffect(() => {
    if (!listRef.current) return;
    const selectedEl = listRef.current.querySelector(
      `[data-index="${selectedIndex}"]`,
    );
    if (selectedEl && typeof selectedEl.scrollIntoView === "function") {
      selectedEl.scrollIntoView({ block: "nearest" });
    }
  }, [selectedIndex]);

  // Reset selection when query changes
  useEffect(() => {
    setSelectedIndex(0);
  }, [query]);

  if (!isOpen) return null;

  // Flatten the grouped list for index tracking
  let flatIndex = 0;

  return (
    <div
      className="fixed inset-0 z-50 flex items-start justify-center pt-[20vh]"
      onClick={onClose}
    >
      {/* Backdrop */}
      <div className="fixed inset-0 bg-black/50" />

      {/* Palette */}
      <div
        className="relative z-10 w-full max-w-lg rounded-xl border border-zinc-700 bg-zinc-900 shadow-2xl"
        onClick={(e) => e.stopPropagation()}
        onKeyDown={handleKeyDown}
      >
        {/* Search input */}
        <div className="flex items-center gap-2 border-b border-zinc-700 px-4 py-3">
          <svg
            className="h-5 w-5 text-zinc-400"
            fill="none"
            stroke="currentColor"
            viewBox="0 0 24 24"
          >
            <path
              strokeLinecap="round"
              strokeLinejoin="round"
              strokeWidth={2}
              d="M21 21l-6-6m2-5a7 7 0 11-14 0 7 7 0 0114 0z"
            />
          </svg>
          <input
            ref={inputRef}
            type="text"
            placeholder="Search commands..."
            aria-label="Search commands"
            value={query}
            onChange={(e) => setQuery(e.target.value)}
            className="flex-1 bg-transparent text-sm text-white placeholder-zinc-500 outline-none"
          />
        </div>

        {/* Results */}
        <div ref={listRef} className="max-h-80 overflow-y-auto p-2">
          {filtered.length === 0 ? (
            <div className="px-4 py-8 text-center text-sm text-zinc-500">
              No matching commands
            </div>
          ) : (
            categoryOrder.map((cat) => {
              const items = grouped[cat];
              if (!items || items.length === 0) return null;
              return (
                <div key={cat} className="mb-2">
                  <div className="px-2 py-1 text-xs font-semibold uppercase text-zinc-500">
                    {categoryLabels[cat]}
                  </div>
                  {items.map((action) => {
                    const idx = flatIndex++;
                    return (
                      <button
                        key={action.id}
                        data-index={idx}
                        className={`flex w-full items-center justify-between rounded-lg px-3 py-2 text-left text-sm ${
                          idx === selectedIndex
                            ? "bg-zinc-700 text-white"
                            : "text-zinc-300 hover:bg-zinc-800"
                        }`}
                        onClick={() => {
                          action.execute();
                          onClose();
                        }}
                        onMouseEnter={() => setSelectedIndex(idx)}
                      >
                        <span>{action.label}</span>
                        {action.shortcut && (
                          <span className="ml-2 text-xs text-zinc-500">
                            {action.shortcut}
                          </span>
                        )}
                      </button>
                    );
                  })}
                </div>
              );
            })
          )}
        </div>

        {/* Footer */}
        <div className="flex items-center gap-4 border-t border-zinc-700 px-4 py-2 text-xs text-zinc-500">
          <span>
            <kbd className="rounded border border-zinc-600 px-1">&uarr;</kbd>{" "}
            <kbd className="rounded border border-zinc-600 px-1">&darr;</kbd>{" "}
            navigate
          </span>
          <span>
            <kbd className="rounded border border-zinc-600 px-1">Enter</kbd>{" "}
            select
          </span>
          <span>
            <kbd className="rounded border border-zinc-600 px-1">Esc</kbd>{" "}
            close
          </span>
        </div>
      </div>
    </div>
  );
};
