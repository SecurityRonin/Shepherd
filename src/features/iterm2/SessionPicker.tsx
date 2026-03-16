import { useState } from 'react';

interface Props {
  taskId: number;
  sessions: string[];
  onResume: (sessionId: string) => void;
  onFresh: () => void;
}

export function SessionPicker({ sessions, onResume, onFresh }: Props) {
  const [selected, setSelected] = useState(sessions[0] ?? '');
  const hasSession = sessions.length > 0;

  return (
    <div className="flex flex-col gap-2">
      {hasSession ? (
        <select
          className="rounded border px-2 py-1 text-sm"
          value={selected}
          onChange={e => setSelected(e.target.value)}
        >
          {sessions.map(s => (
            <option key={s} value={s}>{s}</option>
          ))}
        </select>
      ) : (
        <p className="text-sm text-muted-foreground">No sessions available</p>
      )}
      <div className="flex gap-2">
        <button
          className="rounded bg-blue-600 px-3 py-1 text-sm text-white disabled:opacity-50"
          disabled={!hasSession}
          onClick={() => onResume(selected)}
        >
          Resume
        </button>
        <button
          className="rounded border px-3 py-1 text-sm"
          onClick={onFresh}
        >
          Start Fresh
        </button>
      </div>
    </div>
  );
}
