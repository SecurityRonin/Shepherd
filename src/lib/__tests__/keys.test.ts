import { describe, it, expect, vi } from 'vitest';
import { eventToKeyString, createShortcutManager } from '../keys';

function mockKeyEvent(overrides: Partial<KeyboardEvent> = {}): KeyboardEvent {
  return {
    key: 'n', altKey: false, ctrlKey: false, metaKey: false, shiftKey: false,
    preventDefault: vi.fn(), stopPropagation: vi.fn(),
    ...overrides,
  } as unknown as KeyboardEvent;
}

describe('eventToKeyString', () => {
  it('simple key', () => {
    expect(eventToKeyString(mockKeyEvent({ key: 'a' }))).toBe('a');
  });

  it('meta + key', () => {
    expect(eventToKeyString(mockKeyEvent({ key: 'n', metaKey: true }))).toBe('meta+n');
  });

  it('meta + shift + key', () => {
    expect(eventToKeyString(mockKeyEvent({ key: 'Enter', metaKey: true, shiftKey: true }))).toBe('meta+shift+enter');
  });

  it('number key', () => {
    expect(eventToKeyString(mockKeyEvent({ key: '1' }))).toBe('1');
  });

  it('modifier-only key returns just modifiers', () => {
    expect(eventToKeyString(mockKeyEvent({ key: 'Meta', metaKey: true }))).toBe('meta');
  });

  it('ctrl + key', () => {
    expect(eventToKeyString(mockKeyEvent({ key: 'k', ctrlKey: true }))).toBe('ctrl+k');
  });

  it('bracket keys', () => {
    expect(eventToKeyString(mockKeyEvent({ key: '[', metaKey: true }))).toBe('meta+[');
    expect(eventToKeyString(mockKeyEvent({ key: ']', metaKey: true }))).toBe('meta+]');
  });
});

describe('ShortcutManager', () => {
  it('registers and handles shortcut', () => {
    const manager = createShortcutManager();
    const handler = vi.fn();
    manager.register({ id: 'test', label: 'Test', keys: 'meta+n', handler });

    const event = mockKeyEvent({ key: 'n', metaKey: true });
    const handled = manager.handleKeyDown(event, 'overview');

    expect(handled).toBe(true);
    expect(handler).toHaveBeenCalledOnce();
    expect(event.preventDefault).toHaveBeenCalled();
  });

  it('ignores non-matching keys', () => {
    const manager = createShortcutManager();
    const handler = vi.fn();
    manager.register({ id: 'test', label: 'Test', keys: 'meta+n', handler });

    const handled = manager.handleKeyDown(mockKeyEvent({ key: 'k', metaKey: true }), 'overview');
    expect(handled).toBe(false);
    expect(handler).not.toHaveBeenCalled();
  });

  it('respects viewMode filter', () => {
    const manager = createShortcutManager();
    const handler = vi.fn();
    manager.register({ id: 'test', label: 'Test', keys: '1', viewMode: 'overview', handler });

    // Should work in overview
    manager.handleKeyDown(mockKeyEvent({ key: '1' }), 'overview');
    expect(handler).toHaveBeenCalledOnce();

    // Should not work in focus
    manager.handleKeyDown(mockKeyEvent({ key: '1' }), 'focus');
    expect(handler).toHaveBeenCalledOnce(); // still 1, not called again
  });

  it('unregister removes shortcut', () => {
    const manager = createShortcutManager();
    const handler = vi.fn();
    manager.register({ id: 'test', label: 'Test', keys: 'meta+n', handler });
    manager.unregister('test');

    manager.handleKeyDown(mockKeyEvent({ key: 'n', metaKey: true }), 'overview');
    expect(handler).not.toHaveBeenCalled();
  });

  it('getAll returns all registered shortcuts', () => {
    const manager = createShortcutManager();
    manager.register({ id: 'a', label: 'A', keys: 'meta+a', handler: vi.fn() });
    manager.register({ id: 'b', label: 'B', keys: 'meta+b', handler: vi.fn() });
    expect(manager.getAll()).toHaveLength(2);
  });
});
