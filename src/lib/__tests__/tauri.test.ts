import { describe, it, expect, vi, beforeEach } from 'vitest';

describe('tauri.ts', () => {
  beforeEach(() => {
    vi.resetModules();
    // Clean up __TAURI__ from window
    delete (window as any).__TAURI__;
  });

  describe('invoke', () => {
    it('throws when Tauri is not available', async () => {
      const { invoke } = await import('../tauri');
      await expect(invoke('some_cmd')).rejects.toThrow('Tauri not available');
    });

    it('calls Tauri invoke when __TAURI__ is present', async () => {
      const mockInvoke = vi.fn().mockResolvedValue(42);
      (window as any).__TAURI__ = {};

      // Mock the dynamic import
      vi.doMock('@tauri-apps/api/core', () => ({
        invoke: mockInvoke,
      }));

      const { invoke } = await import('../tauri');
      const result = await invoke('get_port', { key: 'val' });
      expect(result).toBe(42);
      expect(mockInvoke).toHaveBeenCalledWith('get_port', { key: 'val' });
    });
  });

  describe('getServerPort', () => {
    it('returns 9876 fallback when Tauri is not available', async () => {
      const { getServerPort } = await import('../tauri');
      const port = await getServerPort();
      expect(port).toBe(9876);
    });

    it('invokes get_server_port when Tauri is available', async () => {
      const mockInvoke = vi.fn().mockResolvedValue(7532);
      (window as any).__TAURI__ = {};

      vi.doMock('@tauri-apps/api/core', () => ({
        invoke: mockInvoke,
      }));

      const { getServerPort } = await import('../tauri');
      const port = await getServerPort();
      expect(port).toBe(7532);
      expect(mockInvoke).toHaveBeenCalledWith('get_server_port', undefined);
    });

    it('caches the port after first call', async () => {
      const mockInvoke = vi.fn().mockResolvedValue(7532);
      (window as any).__TAURI__ = {};

      vi.doMock('@tauri-apps/api/core', () => ({
        invoke: mockInvoke,
      }));

      const { getServerPort } = await import('../tauri');
      await getServerPort();
      await getServerPort();
      // Should only call invoke once due to caching
      expect(mockInvoke).toHaveBeenCalledTimes(1);
    });
  });
});
