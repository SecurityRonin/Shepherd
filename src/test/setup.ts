import "@testing-library/jest-dom/vitest";

// Polyfill ResizeObserver for jsdom (used by Terminal component)
if (typeof globalThis.ResizeObserver === "undefined") {
  globalThis.ResizeObserver = class ResizeObserver {
    observe() {}
    unobserve() {}
    disconnect() {}
  } as unknown as typeof globalThis.ResizeObserver;
}
