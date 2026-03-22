import { useEffect, useRef } from "react";
import { listen, type UnlistenFn } from "../lib/tauri";

export interface AuthCallbackOptions {
  onSuccess?: (payload: unknown) => void;
  onError?: (payload: unknown) => void;
}

export function useAuthCallback(options: AuthCallbackOptions = {}): void {
  const optionsRef = useRef(options);
  optionsRef.current = options;

  useEffect(() => {
    const unlisteners: Promise<UnlistenFn>[] = [];

    unlisteners.push(
      listen("auth-callback-success", (event) => {
        optionsRef.current.onSuccess?.(event.payload);
      }),
    );

    unlisteners.push(
      listen("auth-callback-error", (event) => {
        optionsRef.current.onError?.(event.payload);
      }),
    );

    return () => {
      unlisteners.forEach((p) => p.then((unlisten) => unlisten()));
    };
  }, []);
}
