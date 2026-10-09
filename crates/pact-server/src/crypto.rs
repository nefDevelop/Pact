//! Utilidades de cifrado simétrico con una clave aportada por el cliente.
//!
//! La contraseña de la base de datos remota se almacena cifrada E2E con una
//! subclave derivada de la MasterKey. El cliente envía esa subclave en cada
//! operación de sincronización (`X-Pact-Sync-Key`) y el servidor solo descifra
//! la contraseña en tránsito, nunca en reposo.

/// Cifra un texto plano con AES-256-GCM en el formato JSON `{nonce,tag,ciphertext}`.
pub fn encrypt_with_key(plaintext: &str, key: &[u8]) -> Result<String, String> {
    pact_core::crypto::sellar_dato_json(plaintext.as_bytes(), key)
        .map_err(|e| format!("{e:?}"))
}

/// Descifra un blob producido por `encrypt_with_key` (o por el cliente con el
/// mismo formato). Devuelve `Ok(String::new())` si el blob está vacío.
pub fn decrypt_with_key(encrypted: &str, key: &[u8]) -> Result<String, String> {
    if encrypted.is_empty() {
        return Ok(String::new());
    }

    let bytes = if encrypted.starts_with('{') {
        pact_core::crypto::abrir_dato_json(encrypted, key).map_err(|e| format!("{e:?}"))?
    } else {
        let raw = hex::decode(encrypted).map_err(|e| e.to_string())?;
        pact_core::crypto::abrir_dato(&raw, key).map_err(|e| format!("{e:?}"))?
    };

    String::from_utf8(bytes).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn key_roundtrip() {
        let key = [7u8; 32];
        let blob = encrypt_with_key("super-secret", &key).unwrap();
        assert!(blob.contains("\"nonce\""));
        assert_eq!(decrypt_with_key(&blob, &key).unwrap(), "super-secret");
    }

    #[test]
    fn wrong_key_fails() {
        let blob = encrypt_with_key("x", &[1u8; 32]).unwrap();
        assert!(decrypt_with_key(&blob, &[2u8; 32]).is_err());
    }

    #[test]
    fn empty_is_empty() {
        assert_eq!(decrypt_with_key("", &[0u8; 32]).unwrap(), "");
    }
}
