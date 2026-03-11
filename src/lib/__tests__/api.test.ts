import { describe, it, expect, vi, beforeEach } from 'vitest';
import { listTasks, createTask, deleteTask, checkHealth, approveTask, ApiError } from '../api';

// Mock fetch globally
const mockFetch = vi.fn();
globalThis.fetch = mockFetch;

beforeEach(() => {
  mockFetch.mockReset();
});

describe('REST API client', () => {
  it('checkHealth returns status', async () => {
    mockFetch.mockResolvedValueOnce({
      ok: true, status: 200,
      json: async () => ({ status: 'ok', version: '0.1.0' }),
    });
    const result = await checkHealth();
    expect(result.status).toBe('ok');
  });

  it('listTasks returns array', async () => {
    mockFetch.mockResolvedValueOnce({
      ok: true, status: 200,
      json: async () => ([{ id: 1, title: 'test', status: 'queued' }]),
    });
    const tasks = await listTasks();
    expect(tasks).toHaveLength(1);
    expect(tasks[0].id).toBe(1);
  });

  it('createTask sends POST', async () => {
    mockFetch.mockResolvedValueOnce({
      ok: true, status: 201,
      json: async () => ({ id: 1, title: 'new task', status: 'queued' }),
    });
    const task = await createTask({ title: 'new task', agent_id: 'claude-code' });
    expect(task.id).toBe(1);
    expect(mockFetch).toHaveBeenCalledWith(
      expect.stringContaining('/api/tasks'),
      expect.objectContaining({ method: 'POST' }),
    );
  });

  it('deleteTask sends DELETE', async () => {
    mockFetch.mockResolvedValueOnce({
      ok: true, status: 200,
      json: async () => ({ deleted: 1 }),
    });
    const result = await deleteTask(1);
    expect(result.deleted).toBe(1);
  });

  it('approveTask sends POST', async () => {
    mockFetch.mockResolvedValueOnce({
      ok: true, status: 200,
      json: async () => ({ status: 'approved' }),
    });
    const result = await approveTask(1);
    expect(result.status).toBe('approved');
  });

  it('throws ApiError on non-ok response', async () => {
    mockFetch.mockResolvedValueOnce({
      ok: false, status: 404,
      json: async () => ({ error: 'not found' }),
    });
    await expect(listTasks()).rejects.toThrow(ApiError);
    await expect(listTasks()).rejects.toThrow(); // reset needed
  });

  it('ApiError has status and body', async () => {
    mockFetch.mockResolvedValueOnce({
      ok: false, status: 500,
      json: async () => ({ error: 'internal' }),
    });
    try {
      await listTasks();
    } catch (err) {
      expect(err).toBeInstanceOf(ApiError);
      expect((err as ApiError).status).toBe(500);
    }
  });
});
