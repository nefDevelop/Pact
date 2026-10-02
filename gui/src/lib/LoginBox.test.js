import { describe, it, expect, vi, beforeEach } from 'vitest';
import { mount, unmount, flushSync } from 'svelte';

// Mocks de las dependencias externas (Tauri y red) para aislar el componente.
vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn(async () => ({
    master_key: 'aa'.repeat(32),
    auth_key: 'bb'.repeat(32),
    sync_key: 'cc'.repeat(32)
  }))
}));

const get = vi.fn();
const post = vi.fn();

vi.mock('./api.js', () => ({
  api: {
    get: (...args) => get(...args),
    post: (...args) => post(...args),
    assetUrl: () => null
  },
  apiAssetUrl: () => null
}));

vi.mock('./crypto_utils.js', () => ({
  deriveKeys: vi.fn(async () => ({
    masterKey: { fake: 'masterKey' },
    authKey: 'authkeyhex',
    syncKey: { fake: 'syncKey' },
    syncKeyHex: 'cc'.repeat(32)
  })),
  // `i18n` importa `safeStorage` de aquí, así que el mock debe exportarlo.
  safeStorage: {
    getItem: () => null,
    setItem: () => {},
    removeItem: () => {}
  }
}));

import LoginBox from './LoginBox.svelte';
import { i18n } from './i18n.svelte.js';
import { globalState } from './state.svelte.js';

function buildProps(overrides = {}) {
  return {
    user: { username: 'alice', name: 'Alice', color: '#6750A4', avatar: '/avatar.png' },
    onUnlock: () => {},
    ...overrides
  };
}

describe('LoginBox', () => {
  let target;

  beforeEach(() => {
    i18n.lang = 'es';
    get.mockReset();
    post.mockReset();
    target = document.createElement('div');
    document.body.appendChild(target);
    globalState.selectedUser = null;
  });

  function mountBox(props = buildProps()) {
    const component = mount(LoginBox, { target, props });
    flushSync();
    return component;
  }

  function cleanup(component) {
    unmount(component);
    target.remove();
  }

  it('renderiza el nombre del usuario', () => {
    const component = mountBox();
    expect(target.textContent).toContain('Alice');
    cleanup(component);
  });

  it('no llama a la API si la contraseña está vacía', () => {
    const onUnlock = vi.fn();
    const component = mountBox(buildProps({ onUnlock }));

    // El botón existe pero sin contraseña no debe disparar nada.
    target.querySelector('button:last-of-type')?.click();
    flushSync();

    expect(get).not.toHaveBeenCalled();
    expect(post).not.toHaveBeenCalled();
    expect(onUnlock).not.toHaveBeenCalled();
    cleanup(component);
  });

  it('deriva las claves y desbloquea con una contraseña válida', async () => {
    get.mockResolvedValue({ salt: 'abcd' });
    post.mockResolvedValue({ ok: true });

    const onUnlock = vi.fn();
    const component = mountBox(buildProps({ onUnlock }));

    const input = target.querySelector('input[type="password"]');
    expect(input).toBeTruthy();

    // Svelte 5 con $state: disparamos el input como lo haría el usuario.
    const valueSetter = Object.getOwnPropertyDescriptor(
      window.HTMLInputElement.prototype,
      'value'
    ).set;
    valueSetter.call(input, 'mi-clave');
    input.dispatchEvent(new Event('input', { bubbles: true }));
    flushSync();

    // El login se dispara con el submit del <form> (evita el reload por defecto).
    const form = target.querySelector('form');
    expect(form).toBeTruthy();
    form.dispatchEvent(new Event('submit', { bubbles: true, cancelable: true }));
    flushSync();

    // Deja correr las promesas encadenadas (salt -> derive -> unlock).
    await new Promise((r) => setTimeout(r, 0));
    await new Promise((r) => setTimeout(r, 0));
    flushSync();

    expect(get).toHaveBeenCalledWith('/auth/salt/alice');
    expect(post).toHaveBeenCalledWith('/auth/unlock', {
      username: 'alice',
      master_password: 'authkeyhex'
    });
    expect(onUnlock).toHaveBeenCalled();

    cleanup(component);
  });
});