import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { globalState } from './state.svelte.js';

// Mock de fetch global
global.fetch = vi.fn();

describe('Global State: Svelte 5 Store', () => {
  beforeEach(() => {
    vi.useFakeTimers();
    vi.clearAllMocks();
    
    // Resetear el estado manualmente ya que es un singleton exportado
    globalState.notifications = [];
    globalState.selectedUser = null;
    globalState.masterKey = null;
    globalState.authToken = null;
    globalState.remainingSeconds = 300;
    if (globalState.inactivityTimer) clearTimeout(globalState.inactivityTimer);
    if (globalState.countdownInterval) clearInterval(globalState.countdownInterval);
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  it('debería añadir y eliminar notificaciones', () => {
    globalState.notify('Test', 'Mensaje', 'info', 0); // 0 para que no se auto-borre
    expect(globalState.notifications.length).toBe(1);
    expect(globalState.notifications[0].title).toBe('Test');

    const id = globalState.notifications[0].id;
    globalState.removeNotification(id);
    expect(globalState.notifications.length).toBe(0);
  });

  it('debería auto-eliminar notificaciones tras el tiempo especificado', () => {
    globalState.notify('Auto', 'Bye', 'success', 1000);
    expect(globalState.notifications.length).toBe(1);

    vi.advanceTimersByTime(1001);
    expect(globalState.notifications.length).toBe(0);
  });

  it('debería resetear el estado de la sesión correctamente', () => {
    globalState.selectedUser = { username: 'testuser', color: '#ff0000' };
    globalState.masterKey = {}; // Mock object
    globalState.authToken = 'token123';
    
    globalState.resetSessionState();
    
    expect(globalState.selectedUser).toBeNull();
    expect(globalState.masterKey).toBeNull();
    expect(globalState.authToken).toBeNull();
    expect(globalState.pendingCount).toBe(0);
  });

  it('debería gestionar el temporizador de inactividad', () => {
    // Simular entorno de navegador
    global.window = { dispatchEvent: vi.fn() };
    
    globalState.autoLockMinutes = 1;
    globalState.resetInactivityTimer();
    
    expect(globalState.remainingSeconds).toBe(60);
    
    // Avanzar 30 segundos
    vi.advanceTimersByTime(30000);
    expect(globalState.remainingSeconds).toBe(30);
    
    // Avanzar hasta el bloqueo (60s + 500ms de margen)
    vi.advanceTimersByTime(31000);
    // Verificar que se llamó a dispatchEvent (dentro de forceLock)
    expect(global.window.dispatchEvent).toHaveBeenCalled();
  });

  it('debería actualizar la paleta tonal (mock de DOM)', () => {
    const mockStyle = { setProperty: vi.fn() };
    global.document = {
      documentElement: {
        style: mockStyle
      }
    };

    globalState.isDark = false;
    globalState.updateTonalPalette('#6750A4');
    
    expect(mockStyle.setProperty).toHaveBeenCalledWith('--md-primary', expect.stringContaining('hsl'));
    expect(mockStyle.setProperty).toHaveBeenCalledWith('--md-background', expect.stringContaining('hsl'));
  });
});
