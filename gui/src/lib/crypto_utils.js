import { invoke } from "@tauri-apps/api/core";

/**
 * Red de seguridad para entornos donde localStorage es nulo o está bloqueado.
 */
export const safeStorage = {
  getItem: (key) => {
    try {
      return (typeof localStorage !== 'undefined' && localStorage) ? localStorage.getItem(key) : null;
    } catch (e) {
      return null;
    }
  },
  setItem: (key, value) => {
    try {
      if (typeof localStorage !== 'undefined' && localStorage) {
        localStorage.setItem(key, value);
      }
    } catch (e) {
      console.warn("Pact: No se pudo escribir en localStorage (bloqueado por el navegador)");
    }
  },
  removeItem: (key) => {
    try {
      if (typeof localStorage !== 'undefined' && localStorage) {
        localStorage.removeItem(key);
      }
    } catch (e) { /* silent */ }
  }
};

const isTauri = typeof window !== 'undefined' && !!(window.__TAURI_INTERNALS__ || window.__TAURI__);

/**
 * Módulo WASM de `pact-core`, cargado solo cuando hace falta (navegador).
 *
 * En Tauri la derivación va por el comando nativo `derive_keys`, así que el
 * WASM no se descarga nunca en la app de escritorio.
 */
let wasmModulePromise = null;
function loadWasm() {
  if (!wasmModulePromise) {
    // El import dinámico mantiene el WASM fuera del chunk inicial: en Tauri
    // jamás se ejecuta y el navegador lo carga al desbloquear la bóveda.
    wasmModulePromise = import('../wasm/pact_core.js');
  }
  return wasmModulePromise;
}

/**
 * Derivación de llaves E2E con Argon2id.
 *
 * Produce tres llaves a partir de la contraseña maestra y el salt:
 * 1. `masterKey`: cifra los secretos E2E. Nunca sale del cliente.
 * 2. `authKey`: autentica contra el servidor (dominio separado por sal).
 * 3. `syncKey`: subclave que envuelve la contraseña de la BD remota.
 *
 * Ambas rutas (Tauri y navegador) ejecutan el **mismo** código Rust: el
 * escritorio vía comando nativo y el navegador vía el WASM de `pact-core`.
 * Devolviendo exactamente el mismo contrato, una bóveda creada en un cliente se
 * abre en el otro.
 */
export async function deriveKeys(password, saltHex) {
  let keys;
  if (isTauri) {
    keys = await invoke('derive_keys', { password, saltHex });
  } else {
    const wasm = await loadWasm();
    const salt = new Uint8Array(
      saltHex.match(/.{1,2}/g).map((byte) => parseInt(byte, 16))
    );
    const derived = wasm.wasm_derivar_claves_dual(password, salt);
    // `free()` libera el bloque de memoria que reservó la llamada en el WASM.
    keys = {
      master_key: derived.master_key,
      auth_key: derived.auth_key,
      sync_key: derived.sync_key
    };
    derived.free();
  }

  const authKey = keys.auth_key;
  const syncKeyHex = keys.sync_key;

  const hexToBytes = (hex) => new Uint8Array(hex.match(/.{1,2}/g).map(byte => parseInt(byte, 16)));

  // Importamos la masterKey (hex) como un CryptoKey de AES-GCM para usar con la Web Crypto API
  const masterKey = await crypto.subtle.importKey(
    'raw',
    hexToBytes(keys.master_key),
    { name: 'AES-GCM' },
    false,
    ['encrypt', 'decrypt']
  );

  // Subclave para envolver la contraseña de la BD remota (solo vive en RAM).
  const syncKey = await crypto.subtle.importKey(
    'raw',
    hexToBytes(syncKeyHex),
    { name: 'AES-GCM' },
    false,
    ['encrypt', 'decrypt']
  );

  return { masterKey, authKey, syncKey, syncKeyHex };
}

/**
 * Cifrado E2E de un objeto JSON usando la MasterKey.
 * Formato: { nonce: hex, ciphertext: hex, tag: hex } compatible con Rust.
 */
export async function encryptE2E(data, masterKey) {
  const enc = new TextEncoder();
  const encodedData = enc.encode(JSON.stringify(data));
  const iv = crypto.getRandomValues(new Uint8Array(12));

  const ciphertextWithTag = await crypto.subtle.encrypt(
    { name: 'AES-GCM', iv },
    masterKey,
    encodedData
  );

  const fullRaw = new Uint8Array(ciphertextWithTag);
  const ciphertext = fullRaw.slice(0, fullRaw.length - 16);
  const tag = fullRaw.slice(fullRaw.length - 16);

  return JSON.stringify({
    nonce: Array.from(iv).map(b => b.toString(16).padStart(2, '0')).join(''),
    ciphertext: Array.from(ciphertext).map(b => b.toString(16).padStart(2, '0')).join(''),
    tag: Array.from(tag).map(b => b.toString(16).padStart(2, '0')).join('')
  });
}

/**
 * Descifrado E2E de un blob JSON usando la MasterKey.
 */
export async function decryptE2E(blobJson, masterKey) {
  try {
    const blob = JSON.parse(blobJson);
    const iv = new Uint8Array(blob.nonce.match(/.{1,2}/g).map(byte => parseInt(byte, 16)));
    const ciphertext = new Uint8Array(blob.ciphertext.match(/.{1,2}/g).map(byte => parseInt(byte, 16)));
    const tag = new Uint8Array(blob.tag.match(/.{1,2}/g).map(byte => parseInt(byte, 16)));

    const combined = new Uint8Array(ciphertext.length + tag.length);
    combined.set(ciphertext);
    combined.set(tag, ciphertext.length);

    const decryptedBuffer = await crypto.subtle.decrypt(
      { name: 'AES-GCM', iv },
      masterKey,
      combined
    );

    return JSON.parse(new TextDecoder().decode(decryptedBuffer));
  } catch (e) {
    console.error("Error al descifrar E2E:", e);
    return null;
  }
}

/**
 * Decodifica un string Base32 (RFC 4648, sin relleno) a bytes.
 */
function base32Decode(input) {
  const alphabet = 'ABCDEFGHIJKLMNOPQRSTUVWXYZ234567';
  const clean = String(input).toUpperCase().replace(/[\s=]/g, '');
  let bits = 0;
  let value = 0;
  const output = [];
  for (const char of clean) {
    const idx = alphabet.indexOf(char);
    if (idx === -1) throw new Error('Base32 inválido');
    value = (value << 5) | idx;
    bits += 5;
    if (bits >= 8) {
      output.push((value >>> (bits - 8)) & 0xff);
      bits -= 8;
    }
  }
  return new Uint8Array(output);
}

/**
 * Extrae el seed de un URI `otpauth://` o lo devuelve tal cual.
 */
export function extractTOTPSeed(raw) {
  const value = String(raw || '').trim();
  if (/^otpauth:\/\//i.test(value)) {
    try {
      return new URL(value).searchParams.get('secret') || '';
    } catch {
      return '';
    }
  }
  return value;
}

/**
 * Genera un código TOTP (HMAC-SHA1) en el cliente. El seed nunca abandona el
 * dispositivo, por lo que el servidor no puede generar códigos (Zero-Knowledge).
 *
 * @param {string} secretBase32 Seed en Base32 o URI otpauth://
 * @param {{digits?: number, period?: number, timestamp?: number}} [options]
 */
export async function generateTOTP(secretBase32, options = {}) {
  const { digits = 6, period = 30, timestamp = Date.now() } = options;
  const seed = extractTOTPSeed(secretBase32);
  if (!seed) throw new Error('Seed TOTP vacío');

  const keyBytes = base32Decode(seed);
  const key = await crypto.subtle.importKey(
    'raw',
    keyBytes,
    { name: 'HMAC', hash: 'SHA-1' },
    false,
    ['sign']
  );

  const counter = Math.floor(timestamp / 1000 / period);
  const buffer = new ArrayBuffer(8);
  const view = new DataView(buffer);
  view.setUint32(0, Math.floor(counter / 0x100000000));
  view.setUint32(4, counter >>> 0);

  const signature = new Uint8Array(await crypto.subtle.sign('HMAC', key, buffer));
  const offset = signature[signature.length - 1] & 0x0f;
  const binary =
    ((signature[offset] & 0x7f) << 24) |
    ((signature[offset + 1] & 0xff) << 16) |
    ((signature[offset + 2] & 0xff) << 8) |
    (signature[offset + 3] & 0xff);
  const otp = binary % (10 ** digits);
  return otp.toString().padStart(digits, '0');
}

/**
 * Genera la huella digital estable del dispositivo.
 * Usa el machine-id del servidor como fuente principal, complementado con
 * señales estables del navegador. Evita fuentes frágiles como canvas fingerprint
 * o screen resolution que cambian con actualizaciones o cambios de monitor.
 */
export async function generateHardwareFingerprint(externalMachineId = null) {
  const parts = [];

  if (externalMachineId) {
    parts.push(externalMachineId);
  } else {
    const ua = navigator.userAgent;
    const stable = ua.match(/(Chrome|Firefox|Safari|Edg)\/[\d]+/);
    parts.push(stable ? stable[0] : 'unknown');
  }
  parts.push(navigator.hardwareConcurrency || 4);

  const msgUint8 = new TextEncoder().encode(parts.join('|'));
  if (!window.isSecureContext || !crypto.subtle) {
    let hash = 0;
    const str = parts.join('|');
    for (let i = 0; i < str.length; i++) hash = ((hash << 5) - hash) + str.charCodeAt(i);
    const fb = new Uint8Array(32);
    for (let i = 0; i < 32; i++) fb[i] = (hash >> (i % 4)) & 0xFF;
    return fb.buffer;
  }

  return await crypto.subtle.digest('SHA-256', msgUint8);
}

/**
 * Cifra un objeto JSON con AES-GCM usando la huella de hardware.
 * Solo para persistir configuración local.
 */
export async function encryptWithHardware(data) {
  const keyBuffer = await generateHardwareFingerprint();
  if (!crypto.subtle) return JSON.stringify({ _pact_fallback: true, data });
  const key = await crypto.subtle.importKey('raw', keyBuffer, { name: 'AES-GCM' }, false, ['encrypt']);
  const iv = crypto.getRandomValues(new Uint8Array(12));
  const encodedData = new TextEncoder().encode(JSON.stringify(data));
  const ciphertext = await crypto.subtle.encrypt({ name: 'AES-GCM', iv }, key, encodedData);
  const combined = new Uint8Array(iv.length + ciphertext.byteLength);
  combined.set(iv); combined.set(new Uint8Array(ciphertext), iv.length);
  return btoa(String.fromCharCode(...combined));
}

export async function decryptWithHardware(encryptedBase64) {
  if (!encryptedBase64) return null;
  try {
    if (encryptedBase64.startsWith('{"_pact_fallback":')) return JSON.parse(encryptedBase64).data;
    if (!crypto.subtle) return null;
    const combined = new Uint8Array(atob(encryptedBase64).split('').map(c => c.charCodeAt(0)));
    const iv = combined.slice(0, 12); const ciphertext = combined.slice(12);
    const keyBuffer = await generateHardwareFingerprint();
    const key = await crypto.subtle.importKey('raw', keyBuffer, { name: 'AES-GCM' }, false, ['decrypt']);
    const decryptedBuffer = await crypto.subtle.decrypt({ name: 'AES-GCM', iv }, key, ciphertext);
    return JSON.parse(new TextDecoder().decode(decryptedBuffer));
  } catch (e) { return null; }
}

