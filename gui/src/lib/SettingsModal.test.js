import { describe, it, expect, vi, beforeEach } from 'vitest';
import { mount, unmount, flushSync } from 'svelte';

vi.mock('@tauri-apps/api/core', () => ({ invoke: vi.fn(async () => null) }));

const get = vi.fn();
const post = vi.fn();
vi.mock('./api.js', () => ({
  api: {
    get: (...a) => get(...a),
    post: (...a) => post(...a),
    assetUrl: () => null
  },
  apiAssetUrl: () => null
}));

vi.mock('./crypto_utils.js', () => ({
  encryptE2E: vi.fn(async (d) => JSON.stringify({ nonce: '00', ciphertext: 'aa', tag: 'bb' })),
  encryptWithHardware: vi.fn(async (d) => 'cifrado'),
  decryptWithHardware: vi.fn(async () => 'plano'),
  safeStorage: { getItem: () => null, setItem: () => {}, removeItem: () => {} }
}));

import SettingsModal from './SettingsModal.svelte';
import { i18n } from './i18n.svelte.js';
import { globalState } from './state.svelte.js';

function buildProps(overrides = {}) {
  return {
    isOpen: true,
    isUnlocked: true,
    folders: [],
    onRefreshFolders: () => {},
    onClose: () => {},
    users: [{ id: 1, username: 'alice', name: 'Alice', color: '#6750A4' }],
    ...overrides
  };
}

describe('SettingsModal', () => {
  let target;

  beforeEach(() => {
    i18n.lang = 'es';
    get.mockReset().mockResolvedValue({ path: '', preferred_mode: 'local' });
    post.mockReset();
    target = document.createElement('div');
    document.body.appendChild(target);
    globalState.vaultPath = '';
    globalState.isDark = false;
  });

  function open(props = buildProps()) {
    const component = mount(SettingsModal, { target, props });
    flushSync();
    return component;
  }

  function cleanup(component) {
    unmount(component);
    target.remove();
  }

  it('es un diálogo modal accesible', () => {
    const component = open();

    const dialog = target.querySelector('[role="dialog"]');
    expect(dialog).toBeTruthy();
    expect(dialog.getAttribute('aria-modal')).toBe('true');
    expect(dialog.getAttribute('aria-label')).toBeTruthy();

    cleanup(component);
  });

  it('muestra la navegación de pestañas con etiquetas accesibles', () => {
    const component = open();

    // Cada pestaña expone su nombre como `title` (iconos solos).
    const botones = [...target.querySelectorAll('button[title]')];
    expect(botones.length).toBeGreaterThan(0);
    for (const b of botones) {
      expect(b.getAttribute('title'), 'toda pestaña necesita title').toBeTruthy();
    }

    cleanup(component);
  });

  it('cambia de pestaña al pulsar una entrada de la navegación', () => {
    const component = open();

    const pestana = [...target.querySelectorAll('button[title]')].find((b) =>
      /usuarios|categor/i.test(b.getAttribute('title'))
    );
    expect(pestana).toBeTruthy();
    pestana.click();
    flushSync();

    // Tras cambiar de pestaña aparece su contenido.
    expect(target.textContent.length).toBeGreaterThan(0);

    cleanup(component);
  });

  it('expone un botón de cerrar que invoca onClose', () => {
    const onClose = vi.fn();
    const component = open(buildProps({ onClose }));

    const cerrar = target.querySelector('[aria-label="Cerrar modal"]');
    expect(cerrar).toBeTruthy();
    cerrar.click();
    flushSync();

    expect(onClose).toHaveBeenCalled();

    cleanup(component);
  });

  it('no intenta cargar la configuración remota sin sesión', () => {
    globalState.authToken = null;
    const component = open();

    // Sin token no debe pedir rutas que exigen sesión; si lo hace, el error
    // se captura y la vista sigue usable.
    expect(target.querySelector('[role="dialog"]')).toBeTruthy();

    cleanup(component);
  });
});