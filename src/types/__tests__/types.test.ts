import { describe, it, expect } from 'vitest';
import type { Task, TaskStatus, CreateTask, KanbanColumnDef, AgentInfo } from '../task';
import { AGENT_COLORS } from '../task';
import type { Session } from '../session';
import type { Permission, PermissionDecision } from '../permission';
import type { ServerEvent, ClientEvent, StatusSnapshot, TaskEvent, PermissionEvent } from '../events';

describe('Task types', () => {
  it('TaskStatus has all 6 values', () => {
    const statuses: TaskStatus[] = ['queued', 'running', 'input', 'review', 'error', 'done'];
    expect(statuses).toHaveLength(6);
  });

  it('Task has required fields', () => {
    const task: Task = {
      id: 1, title: 'test', prompt: '', agent_id: 'claude-code',
      repo_path: '/tmp', branch: 'main', isolation_mode: 'worktree',
      status: 'queued', created_at: '2026-01-01', updated_at: '2026-01-01',
    };
    expect(task.id).toBe(1);
    expect(task.status).toBe('queued');
  });

  it('CreateTask has required and optional fields', () => {
    const minimal: CreateTask = { title: 'test', agent_id: 'claude-code' };
    expect(minimal.title).toBe('test');
    const full: CreateTask = { title: 'test', agent_id: 'claude-code', prompt: 'do stuff', repo_path: '/tmp', isolation_mode: 'worktree' };
    expect(full.prompt).toBe('do stuff');
  });

  it('AGENT_COLORS has 5 agents', () => {
    expect(Object.keys(AGENT_COLORS)).toHaveLength(5);
    expect(AGENT_COLORS['claude-code'].color).toBe('#d97706');
  });

  it('AgentInfo shape', () => {
    const info: AgentInfo = { id: 'claude-code', label: 'Claude', color: '#d97706' };
    expect(info.id).toBe('claude-code');
  });

  it('KanbanColumnDef shape', () => {
    const col: KanbanColumnDef = { id: 'queued', label: 'Queued', tasks: [] };
    expect(col.id).toBe('queued');
  });
});

describe('Session types', () => {
  it('Session has nullable fields', () => {
    const session: Session = { id: 1, task_id: 1, pty_pid: null, terminal_log_path: '/tmp/log', started_at: '2026-01-01', ended_at: null };
    expect(session.pty_pid).toBeNull();
    expect(session.ended_at).toBeNull();
  });
});

describe('Permission types', () => {
  it('PermissionDecision covers all values', () => {
    const decisions: PermissionDecision[] = ['auto', 'approved', 'denied', 'pending'];
    expect(decisions).toHaveLength(4);
  });

  it('Permission has required fields', () => {
    const perm: Permission = { id: 1, task_id: 1, tool_name: 'Bash', tool_args: 'ls', decision: 'pending', rule_matched: null, decided_at: null };
    expect(perm.decision).toBe('pending');
  });
});

describe('Event types', () => {
  it('TaskEvent shape', () => {
    const te: TaskEvent = { id: 1, title: 'test', agent_id: 'claude-code', status: 'queued', branch: 'main', repo_path: '/tmp' };
    expect(te.id).toBe(1);
  });

  it('PermissionEvent shape', () => {
    const pe: PermissionEvent = { id: 1, task_id: 1, tool_name: 'Bash', tool_args: 'ls', decision: 'pending' };
    expect(pe.tool_name).toBe('Bash');
  });

  it('ServerEvent task_created shape', () => {
    const event: ServerEvent = { type: 'task_created', data: { id: 1, title: 'test', agent_id: 'claude-code', status: 'queued', branch: 'main', repo_path: '/tmp' } };
    expect(event.type).toBe('task_created');
  });

  it('ServerEvent status_snapshot shape', () => {
    const snapshot: StatusSnapshot = { tasks: [], pending_permissions: [] };
    const event: ServerEvent = { type: 'status_snapshot', data: snapshot };
    expect(event.type).toBe('status_snapshot');
  });

  it('ClientEvent task_create shape', () => {
    const event: ClientEvent = { type: 'task_create', data: { title: 'test', agent_id: 'claude-code' } };
    expect(event.type).toBe('task_create');
  });

  it('ClientEvent subscribe shape', () => {
    const event: ClientEvent = { type: 'subscribe', data: null };
    expect(event.type).toBe('subscribe');
  });
});
