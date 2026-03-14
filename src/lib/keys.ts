export interface Shortcut {
  id: string;
  label: string;
  keys: string;
  handler: () => void;
  viewMode?: "overview" | "focus";
}

export function eventToKeyString(event: KeyboardEvent): string {
  const parts: string[] = [];
  if (event.altKey) parts.push("alt");
  if (event.ctrlKey) parts.push("ctrl");
  if (event.metaKey) parts.push("meta");
  if (event.shiftKey) parts.push("shift");
  const key = event.key.toLowerCase();
  if (!["alt", "control", "meta", "shift"].includes(key)) {
    parts.push(key);
  }
  return parts.join("+");
}

export interface ShortcutManager {
  register(shortcut: Shortcut): void;
  unregister(id: string): void;
  handleKeyDown(event: KeyboardEvent, currentViewMode: string): boolean;
  getAll(): Shortcut[];
}

export function createShortcutManager(): ShortcutManager {
  const shortcuts = new Map<string, Shortcut>();
  return {
    register(shortcut) { shortcuts.set(shortcut.id, shortcut); },
    unregister(id) { shortcuts.delete(id); },
    handleKeyDown(event: KeyboardEvent, currentViewMode: string) {
      const keyString = eventToKeyString(event);
      for (const shortcut of shortcuts.values()) {
        if (shortcut.keys !== keyString) continue;
        if (shortcut.viewMode && shortcut.viewMode !== currentViewMode) continue;
        event.preventDefault();
        event.stopPropagation();
        shortcut.handler();
        return true;
      }
      return false;
    },
    getAll() { return Array.from(shortcuts.values()); },
  };
}
