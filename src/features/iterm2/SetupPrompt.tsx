interface Props {
  onDismiss: () => void;
}

export function SetupPrompt({ onDismiss }: Props) {
  return (
    <div className="rounded-lg border border-yellow-300 bg-yellow-50 p-4 dark:border-yellow-700 dark:bg-yellow-950">
      <h3 className="font-semibold text-yellow-800 dark:text-yellow-200">
        Enable iTerm2 Integration
      </h3>
      <p className="mt-1 text-sm text-yellow-700 dark:text-yellow-300">
        To adopt existing iTerm2 sessions, install the Shepherd bridge script:
      </p>
      <ol className="mt-2 list-decimal pl-5 text-sm text-yellow-700 dark:text-yellow-300">
        <li>Enable the Python API in iTerm2 → Preferences → General → Magic</li>
        <li>
          Copy{' '}
          <code className="rounded bg-yellow-100 px-1 dark:bg-yellow-900">shepherd-bridge.py</code>{' '}
          to{' '}
          <code className="rounded bg-yellow-100 px-1 dark:bg-yellow-900">
            ~/Library/Application Support/iTerm2/Scripts/AutoLaunch/
          </code>
        </li>
        <li>Restart iTerm2</li>
      </ol>
      <button
        className="mt-3 text-xs text-yellow-600 underline dark:text-yellow-400"
        onClick={onDismiss}
        aria-label="Dismiss"
      >
        Dismiss
      </button>
    </div>
  );
}
