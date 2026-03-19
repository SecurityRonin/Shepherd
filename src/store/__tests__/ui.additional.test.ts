import { describe, it, expect, vi, beforeEach } from 'vitest';
import { createUiSlice, type UiSlice } from '../ui';

function createTestUiSlice(): UiSlice {
  let state: UiSlice;
  const set = (partial: Partial<UiSlice> | ((s: UiSlice) => Partial<UiSlice>)) => {
    if (typeof partial === 'function') {
      Object.assign(state, partial(state));
    } else {
      Object.assign(state, partial);
    }
  };
  const get = () => state;
  state = createUiSlice(set as any, get as any, {} as any);
  return state;
}

describe('UiSlice - terminal output handlers', () => {
  let slice: UiSlice;
  beforeEach(() => { slice = createTestUiSlice(); });

  it('starts with null wsClient', () => {
    expect(slice.wsClient).toBeNull();
  });

  it('setWsClient stores and clears client', () => {
    const mockClient = { connect: vi.fn(), disconnect: vi.fn(), send: vi.fn(), getStatus: vi.fn() };
    slice.setWsClient(mockClient as any);
    expect(slice.wsClient).toBe(mockClient);
    slice.setWsClient(null);
    expect(slice.wsClient).toBeNull();
  });

  it('starts with empty terminalOutputHandlers', () => {
    expect(slice.terminalOutputHandlers.size).toBe(0);
  });

  it('registerTerminalHandler adds a handler for a task', () => {
    const handler = vi.fn();
    slice.registerTerminalHandler(42, handler);
    expect(slice.terminalOutputHandlers.size).toBe(1);
    expect(slice.terminalOutputHandlers.get(42)).toBe(handler);
  });

  it('unregisterTerminalHandler removes a handler', () => {
    const handler = vi.fn();
    slice.registerTerminalHandler(42, handler);
    slice.unregisterTerminalHandler(42);
    expect(slice.terminalOutputHandlers.size).toBe(0);
    expect(slice.terminalOutputHandlers.get(42)).toBeUndefined();
  });

  it('dispatchTerminalOutput calls the registered handler', () => {
    const handler = vi.fn();
    slice.registerTerminalHandler(42, handler);
    slice.dispatchTerminalOutput(42, 'hello world');
    expect(handler).toHaveBeenCalledWith('hello world');
    expect(handler).toHaveBeenCalledTimes(1);
  });

  it('dispatchTerminalOutput is a no-op for unregistered tasks', () => {
    // Should not throw when no handler registered
    slice.dispatchTerminalOutput(999, 'data');
  });

  it('multiple handlers for different tasks work independently', () => {
    const handler1 = vi.fn();
    const handler2 = vi.fn();
    slice.registerTerminalHandler(1, handler1);
    slice.registerTerminalHandler(2, handler2);
    slice.dispatchTerminalOutput(1, 'to task 1');
    slice.dispatchTerminalOutput(2, 'to task 2');
    expect(handler1).toHaveBeenCalledWith('to task 1');
    expect(handler2).toHaveBeenCalledWith('to task 2');
    expect(handler1).toHaveBeenCalledTimes(1);
    expect(handler2).toHaveBeenCalledTimes(1);
  });

  it('toggleView switches to focus when focusedTaskId is set', () => {
    // Set focusedTaskId without entering focus mode
    slice.setFocusedTaskId(10);
    slice.setViewMode('overview');
    slice.toggleView();
    expect(slice.viewMode).toBe('focus');
  });

  it('setFocusedPanel changes panel', () => {
    expect(slice.focusedPanel).toBe('terminal');
    slice.setFocusedPanel('changes');
    expect(slice.focusedPanel).toBe('changes');
  });
});
