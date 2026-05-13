import { describe, it, expect, vi, beforeAll } from 'vitest';
import { webcrypto } from 'node:crypto';

// Mock de Tauri para que los tests funcionen fuera del navegador/app
vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn(async (cmd, args) => {
    if (cmd === 'derive_keys') {
      // Simulamos la derivación (en un test real usaríamos una implementación determinista)
      // Para que los tests de consistencia pasen, retornamos algo basado en el password
      const hash = await webcrypto.subtle.digest('SHA-256', new TextEncoder().encode(args.password + args.saltHex));
      const hex = Array.from(new Uint8Array(hash)).map(b => b.toString(16).padStart(2, '0')).join('');
      return {
        master_key: hex,
        auth_key: hex
      };
    }
  })
}));

import { generateHardwareFingerprint, encryptWithHardware, decryptWithHardware } from './crypto_utils';

// Inyectar WebCrypto en el entorno de JSDOM para los tests
beforeAll(() => {
  if (!global.crypto) {
    global.crypto = webcrypto;
  }
  if (!global.crypto.subtle) {
    Object.defineProperty(global.crypto, 'subtle', {
      value: webcrypto.subtle,
      writable: true
    });
  }
  // Forzar contexto seguro para evitar fallbacks en los tests
  Object.defineProperty(window, 'isSecureContext', { value: true, configurable: true });
});

// Mock de las APIs que JSDOM no tiene perfectas
if (typeof window !== 'undefined') {
  // Mock de AudioContext si no existe en el entorno de test
  window.OfflineAudioContext = window.OfflineAudioContext || class {
    constructor() {}
    createOscillator() { return { connect: () => {}, start: () => {}, frequency: { setValueAtTime: () => {} } }; }
    createDynamicsCompressor() { return { connect: () => {}, threshold: { setValueAtTime: () => {} }, knee: { setValueAtTime: () => {} }, ratio: { setValueAtTime: () => {} }, attack: { setValueAtTime: () => {} }, release: { setValueAtTime: () => {} } }; }
    get destination() { return {}; }
    startRendering() { return Promise.resolve({ getChannelData: () => new Float32Array(5000) }); }
  };
}

describe('Crypto Utils: Cifrado E2E (PBKDF2 + AES-GCM)', () => {
  const password = 'mi-super-secreto-123';
  const salt = 'a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4'; // 16 bytes hex

  it('debería derivar llaves consistentes de forma determinista', async () => {
    const { masterKey: m1, authKey: a1 } = await import('./crypto_utils.js').then(m => m.deriveKeys(password, salt));
    const { masterKey: m2, authKey: a2 } = await import('./crypto_utils.js').then(m => m.deriveKeys(password, salt));

    // Las AuthKeys (Strings Hex) deben ser idénticas
    expect(a1).toBe(a2);
    expect(a1.length).toBe(64); // SHA-256 en Hex = 64 chars

    // Las MasterKeys (CryptoKeys) deben ser válidas
    expect(m1.type).toBe('secret');
    expect(m1.algorithm.name).toBe('AES-GCM');
    expect(m1.usages).toContain('encrypt');
    expect(m1.usages).toContain('decrypt');
  });

  it('debería generar una AuthKey diferente si la contraseña o el salt cambian', async () => {
    const { authKey: a1 } = await import('./crypto_utils.js').then(m => m.deriveKeys(password, salt));
    const { authKey: a2 } = await import('./crypto_utils.js').then(m => m.deriveKeys('otra-clave', salt));
    const { authKey: a3 } = await import('./crypto_utils.js').then(m => m.deriveKeys(password, 'ffffffffffffffffffffffffffffffff'));

    expect(a1).not.toBe(a2);
    expect(a1).not.toBe(a3);
  });

  it('debería cifrar y descifrar un objeto JSON correctamente', async () => {
    const { deriveKeys, encryptE2E, decryptE2E } = await import('./crypto_utils.js');
    const { masterKey } = await deriveKeys(password, salt);
    
    const originalData = { title: 'GitHub', user: 'pact-user', pass: 'pact-pass-2026' };
    
    const blobJson = await encryptE2E(originalData, masterKey);
    expect(typeof blobJson).toBe('string');
    
    // El blob debe ser un JSON con los campos nonce, ciphertext y tag
    const blob = JSON.parse(blobJson);
    expect(blob).toHaveProperty('nonce');
    expect(blob).toHaveProperty('ciphertext');
    expect(blob).toHaveProperty('tag');

    const decrypted = await decryptE2E(blobJson, masterKey);
    expect(decrypted).toEqual(originalData);
  });

  it('debería fallar al descifrar si se usa la llave incorrecta', async () => {
    const { deriveKeys, encryptE2E, decryptE2E } = await import('./crypto_utils.js');
    
    const { masterKey: correctKey } = await deriveKeys(password, salt);
    const { masterKey: wrongKey } = await deriveKeys('clave-equivocada', salt);
    
    const data = { secret: 'datos-sensibles' };
    const blobJson = await encryptE2E(data, correctKey);

    // Intentar descifrar con la llave incorrecta
    const decrypted = await decryptE2E(blobJson, wrongKey);
    expect(decrypted).toBeNull();
  });

  it('debería manejar errores de descifrado si el blob está corrupto', async () => {
    const { deriveKeys, decryptE2E } = await import('./crypto_utils.js');
    const { masterKey } = await deriveKeys(password, salt);
    
    // JSON inválido
    const invalidJson = "{ corrupto }";
    const res1 = await decryptE2E(invalidJson, masterKey);
    expect(res1).toBeNull();

    // JSON válido pero datos hex rotos
    const brokenBlob = JSON.stringify({ nonce: 'ff', ciphertext: '00', tag: '11' });
    const res2 = await decryptE2E(brokenBlob, masterKey);
    expect(res2).toBeNull();
  });
});
