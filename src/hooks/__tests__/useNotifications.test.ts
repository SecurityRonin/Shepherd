import { describe, it, expect, vi, beforeEach } from "vitest";
import { notifyFromServer } from "../useNotifications";

// Mock the sounds module
vi.mock("../../lib/sounds", () => ({
  playSound: vi.fn(),
}));

import { playSound } from "../../lib/sounds";

beforeEach(() => {
  vi.restoreAllMocks();
  // Mock the Notification API
  vi.stubGlobal("Notification", class {
    static permission = "granted";
    constructor(public title: string, public options?: { body: string }) {}
  });
});

describe("notifyFromServer", () => {
  it("creates a browser notification with title and body", () => {
    const notifications: Array<{ title: string; body: string }> = [];
    vi.stubGlobal("Notification", class {
      static permission = "granted";
      constructor(title: string, options?: { body: string }) {
        notifications.push({ title, body: options?.body ?? "" });
      }
    });

    notifyFromServer("info", "Test Title", "Test body text");
    expect(notifications).toHaveLength(1);
    expect(notifications[0].title).toBe("Test Title");
    expect(notifications[0].body).toBe("Test body text");
  });

  it("plays complete sound for info kind", () => {
    notifyFromServer("info", "Done", "All good");
    expect(playSound).toHaveBeenCalledWith("complete");
  });

  it("plays permission sound for warning kind", () => {
    notifyFromServer("warning", "Warning", "Watch out");
    expect(playSound).toHaveBeenCalledWith("permission");
  });

  it("plays error sound for error kind", () => {
    notifyFromServer("error", "Error", "Something broke");
    expect(playSound).toHaveBeenCalledWith("error");
  });

  it("plays permission sound for budget_alert kind", () => {
    notifyFromServer("budget_alert", "Budget", "Over limit");
    expect(playSound).toHaveBeenCalledWith("permission");
  });

  it("plays error sound for gate_failure kind", () => {
    notifyFromServer("gate_failure", "Gate Failed", "Lint failed");
    expect(playSound).toHaveBeenCalledWith("error");
  });

  it("plays complete sound for task_complete kind", () => {
    notifyFromServer("task_complete", "Done", "Task finished");
    expect(playSound).toHaveBeenCalledWith("complete");
  });

  it("does not play sound for unknown kind", () => {
    vi.mocked(playSound).mockClear();
    notifyFromServer("unknown_kind", "Title", "Body");
    expect(playSound).not.toHaveBeenCalled();
  });

  it("still notifies when Notification permission is denied", () => {
    // When denied, Notification constructor is never called but function doesn't throw
    vi.stubGlobal("Notification", class {
      static permission = "denied";
      constructor() { /* not called */ }
    });
    // Should not throw
    expect(() => notifyFromServer("info", "Test", "Body")).not.toThrow();
  });
});
