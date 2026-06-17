import { describe, it, expect, vi, beforeEach } from 'vitest';

const mockState = vi.hoisted(() => ({
  authToken: null,
  forceLock: vi.fn(),
  notify: vi.fn(),
}));

vi.mock('./state.svelte.js', () => ({
  globalState: mockState,
}));

import { api } from './api.js';

describe('ApiClient', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    global.fetch = vi.fn();
    mockState.authToken = null;
  });

  it('debería hacer GET requests', async () => {
    global.fetch.mockResolvedValue({
      ok: true,
      status: 200,
      json: async () => ({ data: 'ok' }),
      text: async () => JSON.stringify({ data: 'ok' }),
    });

    const result = await api.get('/test');
    expect(result).toEqual({ data: 'ok' });
    expect(global.fetch).toHaveBeenCalledWith('/test', expect.objectContaining({ method: 'GET' }));
  });

  it('debería incluir el token de autorización', async () => {
    mockState.authToken = 'test-token-123';
    global.fetch.mockResolvedValue({
      ok: true,
      status: 200,
      json: async () => ({}),
      text: async () => '{}',
    });

    await api.get('/test');
    const headers = global.fetch.mock.calls[0][1].headers;
    expect(headers.get('Authorization')).toBe('Bearer test-token-123');
  });

  it('debería forzar lock en 401', async () => {
    mockState.authToken = 'expired-token';
    global.fetch.mockResolvedValue({
      ok: false,
      status: 401,
      json: async () => ({ error: 'No autorizado' }),
      text: async () => JSON.stringify({ error: 'No autorizado' }),
    });

    await expect(api.get('/test')).rejects.toThrow('Unauthorized');
    expect(mockState.forceLock).toHaveBeenCalled();
  });

  it('debería hacer POST con body JSON', async () => {
    global.fetch.mockResolvedValue({
      ok: true,
      status: 200,
      json: async () => ({ id: 1 }),
      text: async () => JSON.stringify({ id: 1 }),
    });

    const body = { name: 'test' };
    const result = await api.post('/create', body);
    expect(result).toEqual({ id: 1 });
    expect(global.fetch).toHaveBeenCalledWith(
      '/create',
      expect.objectContaining({
        method: 'POST',
        body: JSON.stringify(body),
      })
    );
  });

  it('debería manejar 204 No Content', async () => {
    global.fetch.mockResolvedValue({
      ok: true,
      status: 204,
    });

    const result = await api.delete('/item/1');
    expect(result).toBeNull();
  });

  it('debería notificar errores del servidor', async () => {
    global.fetch.mockResolvedValue({
      ok: false,
      status: 500,
      json: async () => ({ error: 'Error interno' }),
      text: async () => JSON.stringify({ error: 'Error interno' }),
    });

    await expect(api.get('/fail')).rejects.toThrow('Error interno');
    expect(mockState.notify).toHaveBeenCalledWith('Error de Servidor', 'Error interno', 'error');
  });

  it('debería hacer DELETE con body', async () => {
    global.fetch.mockResolvedValue({
      ok: true,
      status: 200,
      json: async () => ({ deleted: true }),
      text: async () => JSON.stringify({ deleted: true }),
    });

    const result = await api.delete('/item/1', { reason: 'test' });
    expect(result).toEqual({ deleted: true });
    expect(global.fetch).toHaveBeenCalledWith(
      '/item/1',
      expect.objectContaining({
        method: 'DELETE',
        body: JSON.stringify({ reason: 'test' }),
      })
    );
  });
});
