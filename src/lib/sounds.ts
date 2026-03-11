const SOUND_FILES = {
  permission: "/sounds/permission.wav",
  complete: "/sounds/complete.wav",
  error: "/sounds/error.wav",
} as const;

export type SoundType = keyof typeof SOUND_FILES;

const VOLUME_KEY = "shepherd-sound-volume";
const ENABLED_KEY = "shepherd-sound-enabled";

let volume = parseFloat(localStorage.getItem(VOLUME_KEY) ?? "0.5");
let enabled = localStorage.getItem(ENABLED_KEY) !== "false";

// Preloaded audio elements
const audioCache: Partial<Record<SoundType, HTMLAudioElement>> = {};

function preload(): void {
  for (const [key, src] of Object.entries(SOUND_FILES)) {
    try {
      const audio = new Audio(src);
      audio.preload = "auto";
      audio.volume = volume;
      audioCache[key as SoundType] = audio;
    } catch {
      // Silently ignore — audio may not be available in test/SSR environments
    }
  }
}

// Preload on import (only in browser)
if (typeof window !== "undefined") {
  preload();
}

export function playSound(type: SoundType): void {
  if (!enabled) return;

  const audio = audioCache[type];
  if (!audio) return;

  try {
    audio.volume = volume;
    audio.currentTime = 0;
    audio.play().catch(() => {
      // Silently handle play failures (e.g., autoplay restrictions)
    });
  } catch {
    // Silently handle errors
  }
}

export function setVolume(v: number): void {
  volume = Math.max(0, Math.min(1, v));
  localStorage.setItem(VOLUME_KEY, String(volume));

  // Update all cached audio elements
  for (const audio of Object.values(audioCache)) {
    if (audio) {
      audio.volume = volume;
    }
  }
}

export function setSoundEnabled(value: boolean): void {
  enabled = value;
  localStorage.setItem(ENABLED_KEY, String(value));
}

export function isSoundEnabled(): boolean {
  return enabled;
}

export function getVolume(): number {
  return volume;
}
