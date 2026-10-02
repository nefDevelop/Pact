import { describe, it, expect, vi, beforeEach } from 'vitest';
import { mount, unmount, flushSync } from 'svelte';

vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn(async () => ({ master_key: 'aa'.repeat(32), auth_key: 'bb'.repeat(32), sync_key: 'cc'.repeat(32) }))
}));

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
  encryptE2E: vi.fn(async (data) => JSON.stringify({ nonce: '00', ciphertext: 'aa', tag: 'bb', _data: data })),
  safeStorage: { getItem: () => null, setItem: () => {}, removeItem: () => {} }
}));

import NewEntryModal from './NewEntryModal.svelte';
import { i18n } from './i18n.svelte.js';
import { globalState } from './state.svelte.js';

function buildProps(overrides = {}) {
  return {
    isOpen: true,
    onSave: () => {},
    onClose: () => {},
    onOpenNewFolder: () => {},
    folders: [],
    mode: 'normal',
    entry: null,
    isUnlocked: true,
    ...overrides
  };
}

describe('NewEntryModal', () => {
  let target;

  beforeEach(() => {
    i18n.lang = 'es';
    get.mockReset();
    post.mockReset();
    target = document.createElement('div');
    document.body.appendChild(target);
    // Sin MasterKey el modal se niega a cifrar: hay que simular una bóveda
    // desbloqueada para poder probar el guardado.
    globalState.masterKey = { fake: 'masterKey' };
  });

  function open(props = buildProps()) {
    const component = mount(NewEntryModal, { target, props });
    flushSync();
    return component;
  }

  function cleanup(component) {
    unmount(component);
    target.remove();
  }

  function setValue(id, value) {
    const el = target.querySelector(`#${id}`);
    expect(el, `no se encontró el campo #${id}`).toBeTruthy();
    const setter = Object.getOwnPropertyDescriptor(window.HTMLInputElement.prototype, 'value').set;
    setter.call(el, value);
    el.dispatchEvent(new Event('input', { bubbles: true }));
    flushSync();
  }

  /** Botón de guardar: el que tiene el texto de creación/edición. */
  function saveButton() {
    return [...target.querySelectorAll('button')].find(
      (b) => /guardar|crear/i.test(b.textContent) && !b.disabled
    );
  }

  it('mantiene deshabilitado el guardado con campos vacíos', () => {
    const component = open();

    const buttons = [...target.querySelectorAll('button')];
    const guardar = buttons.find((b) => /guardar|crear/i.test(b.textContent));
    expect(guardar, 'debe existir el botón de guardar').toBeTruthy();
    expect(guardar.disabled, 'sin datos no se debe poder guardar').toBe(true);

    cleanup(component);
  });

  it('rechaza contraseñas críticas por ser comunes o demasiado cortas', () => {
    const onSave = vi.fn();
    const component = open(buildProps({ onSave }));

    setValue('e-title', 'GitHub');
    setValue('e-user', 'octocat');
    setValue('e-pass', '123'); // < 6 caracteres => crítica

    const guardar = [...target.querySelectorAll('button')].find((b) =>
      /guardar|crear/i.test(b.textContent)
    );
    expect(guardar.disabled, 'una contraseña corta debe bloquear el guardado').toBe(true);
    expect(onSave).not.toHaveBeenCalled();

    cleanup(component);
  });

  it('bloquea el guardado con una semilla TOTP inválida', () => {
    const component = open();

    setValue('e-title', 'GitHub');
    setValue('e-user', 'octocat');
    setValue('e-pass', 'una-clave-larga-y-fuerte');
    setValue('e-2fa', '1111'); // Base32 sólo admite A-Z y 2-7

    const guardar = [...target.querySelectorAll('button')].find((b) =>
      /guardar|crear/i.test(b.textContent)
    );
    expect(guardar.disabled, 'una semilla TOTP inválida debe bloquear el guardado').toBe(true);

    cleanup(component);
  });

  it('guarda cuando los datos son válidos', async () => {
    const onSave = vi.fn();
    const component = open(buildProps({ onSave }));

    setValue('e-title', 'GitHub');
    setValue('e-user', 'octocat');
    setValue('e-pass', 'una-clave-larga-y-fuerte');
    setValue('e-2fa', 'JBSWY3DPEHPK3PXP'); // Base32 válido

    const guardar = saveButton();
    expect(guardar, 'con datos válidos el botón debe estar activo').toBeTruthy();
    guardar.click();
    await new Promise((r) => setTimeout(r, 0));
    flushSync();

    expect(onSave).toHaveBeenCalled();
    const guardado = onSave.mock.calls[0][0];
    // Lo que llega al backend va cifrado (el mock de encryptE2E devuelve un
    // sobre JSON), nunca el texto en claro.
    expect(typeof guardado.title).toBe('string');
    expect(guardado.title).not.toBe('GitHub');
    expect(guardado.password).not.toBe('una-clave-larga-y-fuerte');
    expect(JSON.parse(guardado.title)._data).toBe('GitHub');

    cleanup(component);
  });
});