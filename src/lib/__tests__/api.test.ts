import { describe, it, expect, vi, beforeEach } from 'vitest';
import { listTasks, createTask, deleteTask, checkHealth, approveTask, getReplayEvents, getDetectedPlugins, getClaudeSessions, resumeClaudeSession, startFreshSession, ApiError } from '../api';

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

  it('getReplayEvents sends GET to /api/replay/task/:id', async () => {
    mockFetch.mockResolvedValueOnce({
      ok: true, status: 200,
      json: async () => ([
        { id: 1, task_id: 42, session_id: 1, event_type: "tool_call", summary: "test", content: "", metadata: null, timestamp: "2026-03-20T00:00:00Z" },
      ]),
    });
    const events = await getReplayEvents(42);
    expect(events).toHaveLength(1);
    expect(events[0].task_id).toBe(42);
    expect(mockFetch).toHaveBeenCalledWith(
      expect.stringContaining('/api/replay/task/42'),
      expect.objectContaining({ headers: expect.objectContaining({ 'Content-Type': 'application/json' }) }),
    );
  });

  it('getClaudeSessions sends GET to /api/sessions/:id/claude-sessions', async () => {
    mockFetch.mockResolvedValueOnce({
      ok: true, status: 200,
      json: async () => ({ sessions: ["sess-abc", "sess-def"] }),
    });
    const result = await getClaudeSessions(7);
    expect(result.sessions).toEqual(["sess-abc", "sess-def"]);
    expect(mockFetch).toHaveBeenCalledWith(
      expect.stringContaining('/api/sessions/7/claude-sessions'),
      expect.any(Object),
    );
  });

  it('resumeClaudeSession sends POST to /api/sessions/:id/resume', async () => {
    mockFetch.mockResolvedValueOnce({
      ok: true, status: 200,
      json: async () => ({ status: 'resumed' }),
    });
    await resumeClaudeSession(3, 'sess-xyz');
    expect(mockFetch).toHaveBeenCalledWith(
      expect.stringContaining('/api/sessions/3/resume'),
      expect.objectContaining({
        method: 'POST',
        body: JSON.stringify({ claude_session_id: 'sess-xyz' }),
      }),
    );
  });

  it('startFreshSession sends POST to /api/sessions/:id/fresh', async () => {
    mockFetch.mockResolvedValueOnce({
      ok: true, status: 200,
      json: async () => ({ status: 'started' }),
    });
    await startFreshSession(5);
    expect(mockFetch).toHaveBeenCalledWith(
      expect.stringContaining('/api/sessions/5/fresh'),
      expect.objectContaining({ method: 'POST' }),
    );
  });

  it('getDetectedPlugins sends GET to /api/plugins/detected', async () => {
    mockFetch.mockResolvedValueOnce({
      ok: true, status: 200,
      json: async () => ({ detected: ["claude-code", "aider", "playwright"] }),
    });
    const result = await getDetectedPlugins();
    expect(result.detected).toEqual(["claude-code", "aider", "playwright"]);
    expect(mockFetch).toHaveBeenCalledWith(
      expect.stringContaining('/api/plugins/detected'),
      expect.any(Object),
    );
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
