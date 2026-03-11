export interface Session {
  id: number;
  task_id: number;
  pty_pid: number | null;
  terminal_log_path: string;
  started_at: string;
  ended_at: string | null;
}
