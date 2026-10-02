//! Paridad entre la ruta nativa y el export WASM de `pact-core`.
//!
//! El navegador deriva las claves con el WASM y el escritorio con el comando
//! Tauri, ambos sobre estas mismas funciones. Si divergieran, una bóveda creada
//! en un cliente no se abriría en el otro, así que este test compara ambos
//! caminos byte a byte.
//!
//! Ejecutar con: `wasm-pack test --node crates/pact-core`

use pact_core::crypto::{derivar_claves_dual, derivar_subclave};
use pact_core::wasm_derivar_claves_dual;
use wasm_bindgen_test::*;

fn salt_from_hex(hex: &str) -> Vec<u8> {
    (0..hex.len() / 2)
        .map(|i| u8::from_str_radix(&hex[i * 2..i * 2 + 2], 16).unwrap())
        .collect()
}

/// Vector compartido con el test nativo `native_parity_vector` de `src/lib.rs`.
const PASSWORD: &str = "contraseña-de-paridad";
const SALT_HEX: &str = "00a1b2c3d4e5f60718293a4b5c6d7e8f";

#[wasm_bindgen_test]
fn wasm_matches_native_keys() {
    let salt = salt_from_hex(SALT_HEX);

    // Ruta nativa.
    let (m, a) = derivar_claves_dual(PASSWORD, &salt).unwrap();
    let s = derivar_subclave(&m, b"sync");

    // Ruta WASM (la que consume el navegador).
    let derived = wasm_derivar_claves_dual(PASSWORD, &salt).unwrap();

    assert_eq!(derived.master_key, hex::encode(&m), "master_key diverge");
    assert_eq!(derived.auth_key, hex::encode(&a), "auth_key diverge");
    assert_eq!(derived.sync_key, hex::encode(&s), "sync_key diverge");

    // Cada clave son 32 bytes en hex, y los dominios quedan separados.
    assert_eq!(derived.master_key.len(), 64);
    assert_eq!(derived.auth_key.len(), 64);
    assert_eq!(derived.sync_key.len(), 64);
    assert_ne!(derived.master_key, derived.auth_key);
    assert_ne!(derived.master_key, derived.sync_key);
    assert_ne!(derived.auth_key, derived.sync_key);
}

#[wasm_bindgen_test]
fn wasm_accepts_empty_password() {
    // Argon2id no rechaza una contraseña vacía: deriva claves deterministas.
    // El control de contraseña vacía es responsabilidad de la UI, no del KDF.
    // Este test fija ese comportamiento para que no cambie sin avisar.
    let salt = salt_from_hex(SALT_HEX);
    let vacia = wasm_derivar_claves_dual("", &salt).unwrap();
    assert_eq!(vacia.master_key.len(), 64);

    // Y es determinista: misma entrada, misma salida.
    let vacia2 = wasm_derivar_claves_dual("", &salt).unwrap();
    assert_eq!(vacia.master_key, vacia2.master_key);
}