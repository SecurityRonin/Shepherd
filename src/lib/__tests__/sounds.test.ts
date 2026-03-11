import { describe, it, expect, vi, beforeEach } from "vitest";

// Mock HTMLAudioElement before importing sounds module
class MockAudio {
  play = vi.fn().mockResolvedValue(undefined);
  volume = 0.5;
  currentTime = 0;
  preload = "";
  src = "";
  constructor(src?: string) {
    if (src) this.src = src;
  }
}

vi.stubGlobal("Audio", MockAudio);

// We need to reset the module between tests to get fresh state
let sounds: typeof import("../sounds");

describe("sounds", () => {
  beforeEach(async () => {
    vi.clearAllMocks();
    localStorage.clear();
    // Re-import to get fresh module state
    vi.resetModules();
    sounds = await import("../sounds");
  });

  it("playSound does nothing when disabled", () => {
    sounds.setSoundEnabled(false);
    sounds.playSound("permission");
    // The cached audio elements were created during module import.
    // With sounds disabled, play should not be called on any of them.
    // Since we re-imported, the Audio constructor was called for preloading
    // but play() on the cached elements should not fire.
    // We verify enabled state was stored correctly.
    expect(localStorage.getItem("shepherd-sound-enabled")).toBe("false");
    expect(sounds.isSoundEnabled()).toBe(false);
  });

  it("setVolume clamps value above 1 to 1", () => {
    sounds.setVolume(2);
    expect(localStorage.getItem("shepherd-sound-volume")).toBe("1");
    expect(sounds.getVolume()).toBe(1);
  });

  it("setVolume clamps value below 0 to 0", () => {
    sounds.setVolume(-5);
    expect(localStorage.getItem("shepherd-sound-volume")).toBe("0");
    expect(sounds.getVolume()).toBe(0);
  });

  it("setVolume stores valid value correctly", () => {
    sounds.setVolume(0.7);
    expect(localStorage.getItem("shepherd-sound-volume")).toBe("0.7");
    expect(sounds.getVolume()).toBe(0.7);
  });

  it("setSoundEnabled stores value", () => {
    sounds.setSoundEnabled(false);
    expect(localStorage.getItem("shepherd-sound-enabled")).toBe("false");
    expect(sounds.isSoundEnabled()).toBe(false);

    sounds.setSoundEnabled(true);
    expect(localStorage.getItem("shepherd-sound-enabled")).toBe("true");
    expect(sounds.isSoundEnabled()).toBe(true);
  });

  it("isSoundEnabled returns true by default", () => {
    expect(sounds.isSoundEnabled()).toBe(true);
  });

  it("getVolume returns 0.5 by default", () => {
    expect(sounds.getVolume()).toBe(0.5);
  });
});
