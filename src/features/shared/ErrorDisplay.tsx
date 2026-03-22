interface ErrorDisplayProps {
  message: string | null;
  testId?: string;
  variant?: "light" | "dark";
}

export function ErrorDisplay({ message, testId = "error-display", variant = "light" }: ErrorDisplayProps) {
  if (!message) return null;

  const styles = variant === "dark"
    ? "p-3 rounded bg-red-900/30 border border-red-700 text-sm text-red-300"
    : "p-4 bg-red-50 border border-red-200 rounded-lg text-red-700 text-sm";

  return (
    <div className={styles} data-testid={testId}>
      {message}
    </div>
  );
}
