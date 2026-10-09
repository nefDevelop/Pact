pub mod crypto;

use wasm_bindgen::prelude::*;
use crate::crypto::{derivar_masterkey, sellar_dato, abrir_dato};

#[wasm_bindgen]
pub fn wasm_derivar_masterkey(password: &str, salt: &[u8]) -> Result<Vec<u8>, JsValue> {
    derivar_masterkey(password, salt)
        .map_err(|e| JsValue::from_str(&format!("{:?}", e)))
}

/// Claves derivadas, en **hex**, con la misma forma que el comando Tauri
/// `derive_keys` de `pact-desktop`.
///
/// Que ambas rutas devuelvan exactamente el mismo contrato es lo que garantiza
/// que el navegador y el escritorio deriven byte a byte las mismas claves: si
/// divergieran, una bóveda creada en un cliente no se abriría en el otro.
#[wasm_bindgen(getter_with_clone)]
pub struct DerivedKeys {
    /// Clave maestra de cifrado E2E. Nunca sale del cliente.
    pub master_key: String,
    /// Clave de autenticación (dominio separado por prefijo de sal).
    pub auth_key: String,
    /// Subclave derivada de `master_key` para envolver la config de sync.
    pub sync_key: String,
}

#[wasm_bindgen]
pub fn wasm_derivar_claves_dual(password: &str, salt: &[u8]) -> Result<DerivedKeys, JsValue> {
    let (master_key, auth_key) = crate::crypto::derivar_claves_dual(password, salt)
        .map_err(|e| JsValue::from_str(&format!("{:?}", e)))?;

    // Misma derivación de subclave que el comando Tauri (`b"sync"`).
    let sync_key = crate::crypto::derivar_subclave(&master_key, b"sync");

    Ok(DerivedKeys {
        master_key: hex::encode(master_key),
        auth_key: hex::encode(auth_key),
        sync_key: hex::encode(sync_key),
    })
}

#[wasm_bindgen]
pub fn wasm_sellar_dato(texto_plano: &[u8], llave: &[u8]) -> Result<Vec<u8>, JsValue> {
    sellar_dato(texto_plano, llave)
        .map_err(|e| JsValue::from_str(&format!("{:?}", e)))
}

#[wasm_bindgen]
pub fn wasm_abrir_dato(paquete_cifrado: &[u8], llave: &[u8]) -> Result<Vec<u8>, JsValue> {
    abrir_dato(paquete_cifrado, llave)
        .map_err(|e| JsValue::from_str(&format!("{:?}", e)))
}

#[cfg(test)]
mod tests {
    use super::crypto::*;

    #[test]
    fn test_flow_completo() {
        let password = "contraseña-secreta";
        let salt = b"sal-aleatoria-123";

        let llave = derivar_masterkey(password, salt).unwrap();
        assert_eq!(llave.len(), 32);

        let mensaje = b"Este es un secreto de Pact";
        let cifrado = sellar_dato(mensaje, &llave).unwrap();

        // Empieza con el nonce de 12 bytes
        assert!(cifrado.len() > 12 + mensaje.len());

        let descifrado = abrir_dato(&cifrado, &llave).unwrap();
        assert_eq!(descifrado, mensaje);
    }

    #[test]
    fn test_password_generation() {
        let p1 = generar_password_segura(16);
        let p2 = generar_password_segura(16);
        assert_eq!(p1.len(), 16);
        assert_ne!(p1, p2);
    }

    /// Guarda las claves derivadas por la ruta nativa con un vector fijo.
    /// El test de paridad WASM (W5) usa este mismo vector.
    pub const PARITY_PASSWORD: &str = "contraseña-de-paridad";
    pub const PARITY_SALT_HEX: &str = "00a1b2c3d4e5f60718293a4b5c6d7e8f";

    #[test]
    fn native_parity_vector() {
        let salt: Vec<u8> = (0..PARITY_SALT_HEX.len() / 2)
            .map(|i| u8::from_str_radix(&PARITY_SALT_HEX[i * 2..i * 2 + 2], 16).unwrap())
            .collect();

        let (m, a) = derivar_claves_dual(PARITY_PASSWORD, &salt).unwrap();
        let s = derivar_subclave(&m, b"sync");

        assert_eq!(hex::encode(&m).len(), 64);
        assert_eq!(hex::encode(&a).len(), 64);
        assert_eq!(hex::encode(&s).len(), 64);
        // El prefijo de contexto debe separar la subclave de la maestra.
        assert_ne!(hex::encode(&m), hex::encode(&s));
    }
}