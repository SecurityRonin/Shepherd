import { describe, it, expect, beforeEach } from 'vitest';
import { createTasksSlice, type TasksSlice } from '../tasks';
import { createSessionsSlice, type SessionsSlice } from '../sessions';
import { createUiSlice, type UiSlice } from '../ui';
import type { TaskEvent, PermissionEvent } from '../../types/events';

// Helper to create a standalone slice for testing
function createTestTasksSlice(): TasksSlice {
  let state: TasksSlice;
  const set = (partial: Partial<TasksSlice> | ((s: TasksSlice) => Partial<TasksSlice>)) => {
    if (typeof partial === 'function') {
      Object.assign(state, partial(state));
    } else {
      Object.assign(state, partial);
    }
  };
  const get = () => state;
  state = createTasksSlice(set as any, get as any, {} as any);
  return state;
}

function createTestSessionsSlice(): SessionsSlice {
  let state: SessionsSlice;
  const set = (partial: Partial<SessionsSlice> | ((s: SessionsSlice) => Partial<SessionsSlice>)) => {
    if (typeof partial === 'function') {
      Object.assign(state, partial(state));
    } else {
      Object.assign(state, partial);
    }
  };
  const get = () => state;
  state = createSessionsSlice(set as any, get as any, {} as any);
  return state;
}

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

describe('TasksSlice', () => {
  let slice: TasksSlice;
  beforeEach(() => { slice = createTestTasksSlice(); });

  const mockTaskEvent: TaskEvent = {
    id: 1, title: 'Test task', agent_id: 'claude-code',
    status: 'queued', branch: 'main', repo_path: '/tmp',
  };

  it('starts with empty tasks', () => {
    expect(Object.keys(slice.tasks)).toHaveLength(0);
  });

  it('setTasks populates from TaskEvent array', () => {
    slice.setTasks([mockTaskEvent]);
    expect(Object.keys(slice.tasks)).toHaveLength(1);
    expect(slice.tasks[1].title).toBe('Test task');
  });

  it('upsertTask adds new task', () => {
    slice.upsertTask(mockTaskEvent);
    expect(slice.tasks[1]).toBeDefined();
    expect(slice.tasks[1].status).toBe('queued');
  });

  it('upsertTask updates existing task', () => {
    slice.upsertTask(mockTaskEvent);
    slice.upsertTask({ ...mockTaskEvent, status: 'running' });
    expect(slice.tasks[1].status).toBe('running');
  });

  it('removeTask deletes task', () => {
    slice.upsertTask(mockTaskEvent);
    slice.removeTask(1);
    expect(slice.tasks[1]).toBeUndefined();
  });

  it('getTasksByStatus filters correctly', () => {
    slice.setTasks([
      mockTaskEvent,
      { ...mockTaskEvent, id: 2, status: 'running' },
      { ...mockTaskEvent, id: 3, status: 'queued' },
    ]);
    expect(slice.getTasksByStatus('queued')).toHaveLength(2);
    expect(slice.getTasksByStatus('running')).toHaveLength(1);
  });

  it('permission management works', () => {
    const perm: PermissionEvent = { id: 1, task_id: 1, tool_name: 'Bash', tool_args: 'ls', decision: 'pending' };
    slice.addPendingPermission(perm);
    expect(slice.pendingPermissions).toHaveLength(1);
    expect(slice.getPermissionsForTask(1)).toHaveLength(1);
    slice.removePendingPermission(1);
    expect(slice.pendingPermissions).toHaveLength(0);
  });
});

describe('SessionsSlice', () => {
  let slice: SessionsSlice;
  beforeEach(() => { slice = createTestSessionsSlice(); });

  it('starts with empty sessions', () => {
    expect(Object.keys(slice.sessions)).toHaveLength(0);
  });

  it('setSession adds session', () => {
    slice.setSession(1, { id: 1, task_id: 1, pty_pid: 123, terminal_log_path: '/tmp/log', started_at: '2026-01-01', ended_at: null });
    expect(slice.sessions[1]).toBeDefined();
    expect(slice.getSessionForTask(1)?.pty_pid).toBe(123);
  });

  it('removeSession deletes session', () => {
    slice.setSession(1, { id: 1, task_id: 1, pty_pid: 123, terminal_log_path: '/tmp/log', started_at: '2026-01-01', ended_at: null });
    slice.removeSession(1);
    expect(slice.sessions[1]).toBeUndefined();
  });

  it('clearSessions empties all', () => {
    slice.setSession(1, { id: 1, task_id: 1, pty_pid: 123, terminal_log_path: '/tmp/log', started_at: '2026-01-01', ended_at: null });
    slice.setSession(2, { id: 2, task_id: 2, pty_pid: 456, terminal_log_path: '/tmp/log2', started_at: '2026-01-01', ended_at: null });
    slice.clearSessions();
    expect(Object.keys(slice.sessions)).toHaveLength(0);
  });
});

describe('UiSlice', () => {
  let slice: UiSlice;
  beforeEach(() => { slice = createTestUiSlice(); });

  it('starts in overview mode', () => {
    expect(slice.viewMode).toBe('overview');
    expect(slice.focusedTaskId).toBeNull();
  });

  it('enterFocus sets focus mode and task', () => {
    slice.enterFocus(5);
    expect(slice.viewMode).toBe('focus');
    expect(slice.focusedTaskId).toBe(5);
  });

  it('exitFocus returns to overview', () => {
    slice.enterFocus(5);
    slice.exitFocus();
    expect(slice.viewMode).toBe('overview');
    expect(slice.focusedTaskId).toBeNull();
  });

  it('toggleView switches between modes', () => {
    slice.enterFocus(5);
    slice.exitFocus();
    // Can't toggle to focus without a focusedTaskId
    slice.toggleView();
    expect(slice.viewMode).toBe('overview');
    // Set a focused task and toggle
    slice.enterFocus(5);
    slice.toggleView();
    expect(slice.viewMode).toBe('overview');
  });

  it('dialog and palette state', () => {
    expect(slice.isNewTaskDialogOpen).toBe(false);
    slice.setNewTaskDialogOpen(true);
    expect(slice.isNewTaskDialogOpen).toBe(true);
    expect(slice.isCommandPaletteOpen).toBe(false);
    slice.setCommandPaletteOpen(true);
    expect(slice.isCommandPaletteOpen).toBe(true);
  });
});
