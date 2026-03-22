import { describe, it, expect, vi, beforeEach } from 'vitest';
import { listTasks, createTask, deleteTask, checkHealth, approveTask, getReplayEvents, getDetectedPlugins, getClaudeSessions, resumeClaudeSession, startFreshSession, checkTriggers, dismissTrigger, createPr, generateLogo, exportLogo, generateNames, getNorthStarPhases, executeNorthStarPhase, ApiError } from '../api';

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

  // ── Triggers ──────────────────────────────────────────────────

  it('checkTriggers sends POST with project_dir', async () => {
    mockFetch.mockResolvedValueOnce({
      ok: true, status: 200,
      json: async () => ([{ id: 't1', tool: 'logo_generator', message: 'Try logo gen', action_label: 'Go', action_route: '/logo', priority: 'low' }]),
    });
    const result = await checkTriggers('/my/project');
    expect(result).toHaveLength(1);
    expect(result[0].id).toBe('t1');
    expect(mockFetch).toHaveBeenCalledWith(
      expect.stringContaining('/api/triggers/check'),
      expect.objectContaining({
        method: 'POST',
        body: JSON.stringify({ project_dir: '/my/project' }),
      }),
    );
  });

  it('dismissTrigger sends POST with trigger_id and project_dir', async () => {
    mockFetch.mockResolvedValueOnce({
      ok: true, status: 200,
      json: async () => ({ success: true }),
    });
    await dismissTrigger('t1', '/my/project');
    expect(mockFetch).toHaveBeenCalledWith(
      expect.stringContaining('/api/triggers/dismiss'),
      expect.objectContaining({
        method: 'POST',
        body: JSON.stringify({ trigger_id: 't1', project_dir: '/my/project' }),
      }),
    );
  });

  // ── PR Pipeline ───────────────────────────────────────────────

  it('createPr sends POST to /api/tasks/:id/pr', async () => {
    mockFetch.mockResolvedValueOnce({
      ok: true, status: 200,
      json: async () => ({ success: true, pr_url: 'https://github.com/pr/1', steps: [] }),
    });
    const result = await createPr(42, { base_branch: 'main', auto_commit_message: true, run_gates: true });
    expect(result.pr_url).toBe('https://github.com/pr/1');
    expect(mockFetch).toHaveBeenCalledWith(
      expect.stringContaining('/api/tasks/42/pr'),
      expect.objectContaining({ method: 'POST' }),
    );
  });

  // ── LogoGen ───────────────────────────────────────────────────

  it('generateLogo sends POST to /api/logogen', async () => {
    mockFetch.mockResolvedValueOnce({
      ok: true, status: 200,
      json: async () => ({ variants: [{ index: 0, image_data: 'abc', is_url: false }] }),
    });
    const result = await generateLogo({ product_name: 'Test', style: 'minimal', colors: ['#fff', '#000'] });
    expect(result.variants).toHaveLength(1);
    expect(mockFetch).toHaveBeenCalledWith(
      expect.stringContaining('/api/logogen'),
      expect.objectContaining({ method: 'POST' }),
    );
  });

  it('exportLogo sends POST to /api/logogen/export', async () => {
    mockFetch.mockResolvedValueOnce({
      ok: true, status: 200,
      json: async () => ({ files: [{ path: '/out/logo.png', format: 'png', size_bytes: 1024, dimensions: [512, 512] }] }),
    });
    const result = await exportLogo({ image_base64: 'abc', product_name: 'Test' });
    expect(result.files).toHaveLength(1);
    expect(mockFetch).toHaveBeenCalledWith(
      expect.stringContaining('/api/logogen/export'),
      expect.objectContaining({ method: 'POST' }),
    );
  });

  // ── NameGen ───────────────────────────────────────────────────

  it('generateNames sends POST to /api/namegen', async () => {
    mockFetch.mockResolvedValueOnce({
      ok: true, status: 200,
      json: async () => ({ candidates: [{ name: 'TestName', tagline: null, reasoning: 'good', status: 'all_clear', domains: [], npm_available: true, pypi_available: true, github_available: true, negative_associations: [] }] }),
    });
    const result = await generateNames({ description: 'A test project', vibes: ['modern'], count: 5 });
    expect(result.candidates).toHaveLength(1);
    expect(mockFetch).toHaveBeenCalledWith(
      expect.stringContaining('/api/namegen'),
      expect.objectContaining({ method: 'POST' }),
    );
  });

  // ── NorthStar ─────────────────────────────────────────────────

  it('getNorthStarPhases sends GET to /api/northstar/phases', async () => {
    mockFetch.mockResolvedValueOnce({
      ok: true, status: 200,
      json: async () => ({ phases: [{ id: 1, name: 'Discovery', description: 'Phase 1', document_count: 3 }], total: 1 }),
    });
    const result = await getNorthStarPhases();
    expect(result.phases).toHaveLength(1);
    expect(result.phases[0].name).toBe('Discovery');
    expect(mockFetch).toHaveBeenCalledWith(
      expect.stringContaining('/api/northstar/phases'),
      expect.any(Object),
    );
  });

  it('executeNorthStarPhase sends POST to /api/northstar/phase', async () => {
    mockFetch.mockResolvedValueOnce({
      ok: true, status: 200,
      json: async () => ({ phase_id: 1, phase_name: 'Discovery', status: 'completed', output: 'Done', documents: [] }),
    });
    const result = await executeNorthStarPhase({
      product_name: 'Shepherd',
      product_description: 'Agent orchestrator',
      phase_id: 1,
    });
    expect(result.phase_name).toBe('Discovery');
    expect(mockFetch).toHaveBeenCalledWith(
      expect.stringContaining('/api/northstar/phase'),
      expect.objectContaining({ method: 'POST' }),
    );
  });
});
