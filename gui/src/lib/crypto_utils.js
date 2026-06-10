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

/**
 * Derivación de llaves E2E usando Argon2id (vía Rust/Tauri).
 * Genera dos llaves:
 * 1. masterKey: Para cifrar los datos localmente (Nunca sale del cliente).
 * 2. authKey: Para autenticarse con el servidor (Hash derivado).
 */
export async function deriveKeys(password, saltHex) {
  const t0 = performance.now();
  console.log("[🏎️ PERFORMANCE] Iniciando deriveKeys (JS -> Rust bridge)...");
  
  // Llamamos al comando de Rust para realizar el pesado Argon2id
  const keys = await invoke('derive_keys', { password, saltHex });

  const t1 = performance.now();
  console.log(`[🏎️ PERFORMANCE] deriveKeys (Rust Bridge) retornado en ${(t1 - t0).toFixed(2)}ms`);

  const authKey = keys.auth_key;

  // Importamos la masterKey (hex) como un CryptoKey de AES-GCM para usar con la Web Crypto API
  const masterKeyBytes = new Uint8Array(keys.master_key.match(/.{1,2}/g).map(byte => parseInt(byte, 16)));

  const masterKey = await crypto.subtle.importKey(
    'raw',
    masterKeyBytes,
    { name: 'AES-GCM' },
    false,
    ['encrypt', 'decrypt']
  );

  const t2 = performance.now();
  console.log(`[🏎️ PERFORMANCE] Importación masterKey (WebCrypto) completada en ${(t2 - t1).toFixed(2)}ms. Total: ${(t2 - t0).toFixed(2)}ms`);

  return { masterKey, authKey };
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
 * Genera la huella digital del hardware (Audio + Canvas + Sistema + MachineID).
 * Silencioso y persistente en el mismo ordenador.
 * Mantenido solo para proteger el localStorage local.
 */
export async function generateHardwareFingerprint(externalMachineId = null) {
  const parts = [];

  if (externalMachineId) parts.push(externalMachineId);
  parts.push(navigator.hardwareConcurrency || 8);
  parts.push(navigator['deviceMemory'] || 8);
  parts.push(`${screen.width}x${screen.height}x${screen.colorDepth}`);
  parts.push(Intl.DateTimeFormat().resolvedOptions().timeZone);

  try {
    const canvas = document.createElement('canvas');
    const ctx = canvas.getContext('2d');
    canvas.width = 200; canvas.height = 50;
    ctx.textBaseline = "alphabetic"; ctx.fillStyle = "#f60"; ctx.fillRect(125, 1, 62, 20);
    ctx.fillStyle = "#069"; ctx.fillText("pact_vault_fingerprint_2026", 2, 15);
    parts.push(canvas.toDataURL());
  } catch (e) { parts.push('canvas_error'); }

  const msgUint8 = new TextEncoder().encode(parts.join('|'));
  if (!window.isSecureContext || !crypto.subtle) {
    let hash = 0; const str = parts.join('|');
    for (let i = 0; i < str.length; i++) hash = ((hash << 5) - hash) + str.charCodeAt(i);
    const fb = new Uint8Array(32); for (let i = 0; i < 32; i++) fb[i] = (hash >> (i % 4)) & 0xFF;
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

