import React from "react";

interface LazyFallbackProps {
  label?: string;
  testId?: string;
}

export const LazyFallback: React.FC<LazyFallbackProps> = ({
  label = "Loading...",
  testId = "lazy-fallback",
}) => (
  <div
    className="flex-1 flex items-center justify-center bg-shepherd-bg animate-pulse"
    data-testid={testId}
  >
    <span className="text-sm text-shepherd-muted">{label}</span>
  </div>
);
