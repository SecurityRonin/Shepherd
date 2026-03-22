import { useState, useCallback } from "react";

interface AsyncActionState<T> {
  loading: boolean;
  error: string | null;
  data: T | null;
}

interface AsyncActionResult<T, A extends unknown[]> extends AsyncActionState<T> {
  execute: (...args: A) => Promise<void>;
  reset: () => void;
}

export function useAsyncAction<T, A extends unknown[] = unknown[]>(
  action: (...args: A) => Promise<T>,
): AsyncActionResult<T, A> {
  const [state, setState] = useState<AsyncActionState<T>>({
    loading: false,
    error: null,
    data: null,
  });

  const execute = useCallback(
    async (...args: A) => {
      setState({ loading: true, error: null, data: null });
      try {
        const result = await action(...args);
        setState({ loading: false, error: null, data: result });
      } catch (err) {
        setState({
          loading: false,
          error: err instanceof Error ? err.message : "Unknown error",
          data: null,
        });
      }
    },
    [action],
  );

  const reset = useCallback(() => {
    setState({ loading: false, error: null, data: null });
  }, []);

  return { ...state, execute, reset };
}
