import { describe, it, expect, vi, beforeEach } from 'vitest';
import { createTasksSlice, type TasksSlice } from '../tasks';

// Mock the API module
vi.mock('../../lib/api', () => ({
  createTask: vi.fn(),
  listTasks: vi.fn(),
}));

import { createTask as apiCreateTask, listTasks as apiListTasks } from '../../lib/api';

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

describe('TasksSlice - async actions', () => {
  let slice: TasksSlice;

  beforeEach(() => {
    slice = createTestTasksSlice();
    vi.mocked(apiCreateTask).mockReset();
    vi.mocked(apiListTasks).mockReset();
  });

  it('createTask calls API and adds task to store', async () => {
    const mockTask = {
      id: 1, title: 'New task', agent_id: 'claude-code', status: 'queued' as const,
      prompt: '', isolation_mode: 'worktree' as const, branch: '', repo_path: '',
      created_at: '2026-01-01', updated_at: '2026-01-01',
    };
    vi.mocked(apiCreateTask).mockResolvedValue(mockTask);

    const result = await slice.createTask({ title: 'New task', agent_id: 'claude-code' });
    expect(result).toEqual(mockTask);
    expect(slice.tasks[1]).toEqual(mockTask);
    expect(apiCreateTask).toHaveBeenCalledWith({ title: 'New task', agent_id: 'claude-code' });
  });

  it('fetchTasks calls API and populates store', async () => {
    const mockTasks = [
      { id: 1, title: 'Task 1', agent_id: 'claude', status: 'queued' as const, prompt: '', isolation_mode: 'worktree', branch: '', repo_path: '', created_at: '2026-01-01', updated_at: '2026-01-01' },
      { id: 2, title: 'Task 2', agent_id: 'claude', status: 'running' as const, prompt: '', isolation_mode: 'worktree', branch: '', repo_path: '', created_at: '2026-01-01', updated_at: '2026-01-01' },
    ];
    vi.mocked(apiListTasks).mockResolvedValue(mockTasks);

    await slice.fetchTasks();
    expect(Object.keys(slice.tasks)).toHaveLength(2);
    expect(slice.tasks[1].title).toBe('Task 1');
    expect(slice.tasks[2].title).toBe('Task 2');
    expect(apiListTasks).toHaveBeenCalledTimes(1);
  });

  it('fetchTasks replaces existing tasks', async () => {
    // Add a task first via upsertTask
    slice.upsertTask({ id: 99, title: 'Old', agent_id: 'x', status: 'queued', branch: '', repo_path: '' });
    expect(slice.tasks[99]).toBeDefined();

    // Fetch replaces all
    vi.mocked(apiListTasks).mockResolvedValue([
      { id: 1, title: 'Fresh', agent_id: 'claude', status: 'queued' as const, prompt: '', isolation_mode: 'worktree', branch: '', repo_path: '', created_at: '2026-01-01', updated_at: '2026-01-01' },
    ]);
    await slice.fetchTasks();
    expect(slice.tasks[99]).toBeUndefined();
    expect(slice.tasks[1].title).toBe('Fresh');
  });

  it('setTaskDiffs adds diffs to a task', () => {
    slice.upsertTask({ id: 1, title: 'T', agent_id: 'x', status: 'queued', branch: '', repo_path: '' });
    const diffs = [{ file_path: 'src/main.rs', before_content: 'old', after_content: 'new', language: 'rust' }];
    slice.setTaskDiffs(1, diffs);
    expect(slice.tasks[1].diffs).toEqual(diffs);
  });

  it('setTaskDiffs on nonexistent task is a no-op', () => {
    const before = { ...slice.tasks };
    slice.setTaskDiffs(999, []);
    expect(slice.tasks).toEqual(before);
  });

  it('setPendingPermissions replaces all permissions', () => {
    const perms = [
      { id: 1, task_id: 1, tool_name: 'Bash', tool_args: 'ls', decision: 'pending' },
      { id: 2, task_id: 1, tool_name: 'Write', tool_args: '/tmp/x', decision: 'pending' },
    ];
    slice.setPendingPermissions(perms);
    expect(slice.pendingPermissions).toHaveLength(2);
  });

  it('getTaskById returns undefined for missing task', () => {
    expect(slice.getTaskById(999)).toBeUndefined();
  });

  it('getTaskById returns task when present', () => {
    slice.upsertTask({ id: 7, title: 'Found', agent_id: 'x', status: 'queued', branch: '', repo_path: '' });
    expect(slice.getTaskById(7)?.title).toBe('Found');
  });
});
