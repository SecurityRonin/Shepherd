// Tauri invoke wrapper — falls back to HTTP when not in Tauri context
const isTauri = typeof window !== 'undefined' && '__TAURI__' in window;

export async function invoke<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  if (isTauri) {
    const { invoke } = await import('@tauri-apps/api/core');
    return invoke<T>(cmd, args);
  }
  throw new Error(`Tauri not available for command: ${cmd}`);
}

export type UnlistenFn = () => void;

export async function listen<T>(event: string, handler: (event: { payload: T }) => void): Promise<UnlistenFn> {
  if (isTauri) {
    try {
      const { listen: tauriListen } = await import('@tauri-apps/api/event');
      return await tauriListen<T>(event, handler);
    } catch {
      return () => {};
    }
  }
  return () => {};
}

let cachedPort: number | null = null;

export async function getServerPort(): Promise<number> {
  if (cachedPort !== null) return cachedPort;
  if (isTauri) {
    cachedPort = await invoke<number>('get_server_port');
    return cachedPort;
  }
  return 9876; // fallback for dev
}
