import { describe, it, expect, vi, beforeEach } from 'vitest';

// Mock the tauri module to control getServerPort
vi.mock('../tauri', () => ({
  getServerPort: vi.fn().mockResolvedValue(9876),
}));

import { cancelTask, waitForServer, getTask, ApiError } from '../api';
import { getServerPort } from '../tauri';

const mockFetch = vi.fn();
globalThis.fetch = mockFetch;

beforeEach(() => {
  mockFetch.mockReset();
  vi.mocked(getServerPort).mockResolvedValue(9876);
});

describe('api.ts additional coverage', () => {
  it('getBaseUrl uses getServerPort for dynamic port', async () => {
    vi.mocked(getServerPort).mockResolvedValue(4321);
    mockFetch.mockResolvedValueOnce({
      ok: true, status: 200,
      json: async () => ({ id: 1 }),
    });
    await getTask(1);
    expect(mockFetch).toHaveBeenCalledWith(
      'http://127.0.0.1:4321/api/tasks/1',
      expect.anything(),
    );
  });

  it('cancelTask sends POST to cancel endpoint', async () => {
    mockFetch.mockResolvedValueOnce({
      ok: true, status: 200,
      json: async () => ({ status: 'cancelled' }),
    });
    const result = await cancelTask(5);
    expect(result.status).toBe('cancelled');
    expect(mockFetch).toHaveBeenCalledWith(
      expect.stringContaining('/api/tasks/5/cancel'),
      expect.objectContaining({ method: 'POST' }),
    );
  });

  it('waitForServer returns true when health succeeds', async () => {
    mockFetch.mockResolvedValueOnce({
      ok: true, status: 200,
      json: async () => ({ status: 'ok', version: '0.1.0' }),
    });
    const result = await waitForServer(1000, 100);
    expect(result).toBe(true);
  });

  it('waitForServer returns false after timeout', async () => {
    mockFetch.mockRejectedValue(new Error('connection refused'));
    const result = await waitForServer(200, 50);
    expect(result).toBe(false);
  });

  it('request handles 204 No Content', async () => {
    mockFetch.mockResolvedValueOnce({
      ok: true, status: 204,
      json: async () => { throw new Error('no body'); },
    });
    // cancelTask goes through request() -- if server returns 204
    const result = await cancelTask(1);
    expect(result).toBeUndefined();
  });

  it('ApiError falls back to text body when JSON parsing fails', async () => {
    mockFetch.mockResolvedValueOnce({
      ok: false, status: 500,
      json: async () => { throw new Error('not json'); },
      text: async () => 'Internal Server Error',
    });
    try {
      await getTask(1);
    } catch (err) {
      expect(err).toBeInstanceOf(ApiError);
      expect((err as ApiError).body).toBe('Internal Server Error');
    }
  });
});
