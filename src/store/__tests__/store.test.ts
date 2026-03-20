import { describe, it, expect, beforeEach } from 'vitest';
import { createTasksSlice, type TasksSlice } from '../tasks';
import { createSessionsSlice, type SessionsSlice } from '../sessions';
import { createUiSlice, type UiSlice } from '../ui';
import { createObservabilitySlice, type ObservabilitySlice } from '../observability';
import type { TaskEvent, PermissionEvent, MetricsUpdateEvent, BudgetAlertEvent, GateResultEvent } from '../../types/events';

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

function createTestObservabilitySlice(): ObservabilitySlice {
  let state: ObservabilitySlice;
  const set = (partial: Partial<ObservabilitySlice> | ((s: ObservabilitySlice) => Partial<ObservabilitySlice>)) => {
    if (typeof partial === 'function') {
      Object.assign(state, partial(state));
    } else {
      Object.assign(state, partial);
    }
  };
  const get = () => state;
  state = createObservabilitySlice(set as any, get as any, {} as any);
  return state;
}

describe('ObservabilitySlice', () => {
  let slice: ObservabilitySlice;
  beforeEach(() => { slice = createTestObservabilitySlice(); });

  it('starts with empty state', () => {
    expect(slice.agentMetrics).toHaveLength(0);
    expect(slice.spendingSummary).toBeNull();
    expect(slice.gateResults).toEqual({});
    expect(slice.budgetAlerts).toHaveLength(0);
  });

  it('handleMetricsUpdate adds and deduplicates by task_id', () => {
    const event: MetricsUpdateEvent = {
      task_id: 1, agent_id: 'claude-code', model_id: 'claude-sonnet-4',
      total_input_tokens: 5000, total_output_tokens: 2000, total_tokens: 7000,
      total_cost_usd: 0.05, llm_calls: 3, duration_secs: 12.5,
    };
    slice.handleMetricsUpdate(event);
    expect(slice.agentMetrics).toHaveLength(1);
    expect(slice.agentMetrics[0].task_id).toBe(1);
    expect(slice.agentMetrics[0].total_cost_usd).toBe(0.05);

    // Update same task — should replace, not duplicate
    slice.handleMetricsUpdate({ ...event, total_cost_usd: 0.10 });
    expect(slice.agentMetrics).toHaveLength(1);
    expect(slice.agentMetrics[0].total_cost_usd).toBe(0.10);
  });

  it('handleGateResult accumulates per task', () => {
    const gate1: GateResultEvent = { task_id: 1, gate: 'lint', passed: true };
    const gate2: GateResultEvent = { task_id: 1, gate: 'test', passed: false };
    const gate3: GateResultEvent = { task_id: 2, gate: 'lint', passed: true };

    slice.handleGateResult(gate1);
    slice.handleGateResult(gate2);
    slice.handleGateResult(gate3);

    expect(slice.gateResults[1]).toHaveLength(2);
    expect(slice.gateResults[2]).toHaveLength(1);
    expect(slice.gateResults[1][0].gate).toBe('lint');
    expect(slice.gateResults[1][1].passed).toBe(false);
  });

  it('handleBudgetAlert accumulates alerts', () => {
    const alert: BudgetAlertEvent = {
      scope: 'task', scope_id: '1', status: '"exceeded"',
      current_cost: 5.5, limit: 5.0, percentage: 1.1,
      message: 'Budget exceeded: $5.50 / $5.00 (110%)',
    };
    slice.handleBudgetAlert(alert);
    expect(slice.budgetAlerts).toHaveLength(1);
    expect(slice.budgetAlerts[0].scope).toBe('task');
    expect(slice.budgetAlerts[0].current_cost).toBe(5.5);

    slice.handleBudgetAlert({ ...alert, scope: 'agent_daily' });
    expect(slice.budgetAlerts).toHaveLength(2);
  });

  it('setSpendingSummary replaces summary', () => {
    const summary = {
      total_cost_usd: 1.23, total_tokens: 50000, total_tasks: 5,
      total_llm_calls: 15, by_agent: [], by_model: [],
    };
    slice.setSpendingSummary(summary);
    expect(slice.spendingSummary).not.toBeNull();
    expect(slice.spendingSummary!.total_cost_usd).toBe(1.23);

    slice.setSpendingSummary(null);
    expect(slice.spendingSummary).toBeNull();
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
