use argon2::{
    Argon2, Params,
};
use aes_gcm::{
    aead::{Aead, KeyInit},
    Aes256Gcm, Nonce,
};
use blake2::{Blake2b512, Digest};
use rand::{rngs::OsRng, RngCore, Rng};
use std::string::FromUtf8Error;
use serde::{Serialize, Deserialize};
use zeroize::Zeroize;

// Parámetros de alta seguridad (Standard Argon2id)
pub const ARGON2_TIME: u32 = 3;
pub const ARGON2_MEMORY: u32 = 65536; // 64MB
pub const ARGON2_PARALLELISM: u32 = 4;
pub const KEY_LEN: usize = 32;

#[derive(Serialize, Deserialize)]
pub struct EncryptedBlob {
    pub nonce: String,
    pub tag: String,
    pub ciphertext: String,
}

#[derive(Debug)]
pub enum CryptoError {
    KdfError(String),
    CipherError(String),
    EncodingError(FromUtf8Error),
}

/// Derivación de Llave Maestra usando Argon2id.
/// Convierte password y salt en una llave de 256 bits.
pub fn derivar_masterkey(password: &str, salt: &[u8]) -> Result<Vec<u8>, CryptoError> {
    let params = Params::new(ARGON2_MEMORY, ARGON2_TIME, ARGON2_PARALLELISM, Some(KEY_LEN))
        .map_err(|e| CryptoError::KdfError(e.to_string()))?;
    
    let argon2 = Argon2::new(argon2::Algorithm::Argon2id, argon2::Version::V0x13, params);
    
    let mut key = vec![0u8; KEY_LEN];
    argon2
        .hash_password_into(password.as_bytes(), salt, &mut key)
        .map_err(|e| CryptoError::KdfError(e.to_string()))?;
    
    Ok(key)
}

/// Derivación dual: genera MasterKey (para cifrado) y AuthKey (para servidor).
pub fn derivar_claves_dual(password: &str, salt: &[u8]) -> Result<(Vec<u8>, Vec<u8>), CryptoError> {
    // 1. MasterKey: Usa el salt base
    let master_key = derivar_masterkey(password, salt)?;

    // 2. AuthKey: Usa salt con prefijo de contexto (igual que la lógica original de JS)
    let mut auth_salt = b"pact_auth_v1_".to_vec();
    auth_salt.extend_from_slice(salt);
    let auth_key = derivar_masterkey(password, &auth_salt)?;

    // Limpiamos el salt temporal de la memoria (opcional pero recomendado)
    auth_salt.zeroize();

    Ok((master_key, auth_key))
}

/// Cifrado Autenticado AES-256-GCM.
/// Retorna [nonce(12b)] + [ciphertext] + [tag(16b)]
pub fn sellar_dato(texto_plano: &[u8], llave: &[u8]) -> Result<Vec<u8>, CryptoError> {
    let cipher = Aes256Gcm::new_from_slice(llave)
        .map_err(|e| CryptoError::CipherError(e.to_string()))?;
    
    let mut nonce_bytes = [0u8; 12];
    OsRng.fill_bytes(&mut nonce_bytes);
    let nonce = Nonce::from_slice(&nonce_bytes);
    
    let ciphertext = cipher
        .encrypt(nonce, texto_plano)
        .map_err(|e| CryptoError::CipherError(e.to_string()))?;
    
    // Concatenamos nonce + ciphertext (que ya incluye el tag al final en RustCrypto)
    let mut resultado = nonce_bytes.to_vec();
    resultado.extend_from_slice(&ciphertext);
    
    Ok(resultado)
}

/// Helper para compatibilidad con el formato JSON-Hex de Python.
pub fn sellar_dato_json(texto_plano: &[u8], llave: &[u8]) -> Result<String, CryptoError> {
    let raw = sellar_dato(texto_plano, llave)?;
    
    // raw = [nonce(12)] + [ciphertext] + [tag(16)]
    let len = raw.len();
    if len < 28 {
        return Err(CryptoError::CipherError("Blob cifrado inválido".to_string()));
    }
    
    let blob = EncryptedBlob {
        nonce: hex::encode(&raw[0..12]),
        tag: hex::encode(&raw[len - 16..]),
        ciphertext: hex::encode(&raw[12..len - 16]),
    };
    
    serde_json::to_string(&blob).map_err(|e| CryptoError::CipherError(e.to_string()))
}

/// Descifra un campo guardado en formato JSON-Hex.
pub fn abrir_dato_json(json_blob: &str, llave: &[u8]) -> Result<Vec<u8>, CryptoError> {
    let blob: EncryptedBlob = serde_json::from_str(json_blob)
        .map_err(|e| CryptoError::CipherError(e.to_string()))?;
    
    let mut raw = hex::decode(&blob.nonce).map_err(|e| CryptoError::CipherError(e.to_string()))?;
    let mut ciphertext = hex::decode(&blob.ciphertext).map_err(|e| CryptoError::CipherError(e.to_string()))?;
    let tag = hex::decode(&blob.tag).map_err(|e| CryptoError::CipherError(e.to_string()))?;
    
    raw.append(&mut ciphertext);
    raw.extend_from_slice(&tag);
    
    abrir_dato(&raw, llave)
}

/// Descifra y verifica integridad.
pub fn abrir_dato(paquete_cifrado: &[u8], llave: &[u8]) -> Result<Vec<u8>, CryptoError> {
    if paquete_cifrado.len() < 12 {
        return Err(CryptoError::CipherError("Paquete cifrado demasiado corto".to_string()));
    }
    
    let cipher = Aes256Gcm::new_from_slice(llave)
        .map_err(|e| CryptoError::CipherError(e.to_string()))?;
    
    let (nonce_bytes, ciphertext) = paquete_cifrado.split_at(12);
    let nonce = Nonce::from_slice(nonce_bytes);
    
    let texto_plano = cipher
        .decrypt(nonce, ciphertext)
        .map_err(|e| CryptoError::CipherError(e.to_string()))?;
    
    Ok(texto_plano)
}

/// BLAKE2b para verificar integridad de archivos o base de datos.
pub fn hash_integridad(datos: &[u8]) -> String {
    let mut hasher = Blake2b512::new();
    hasher.update(datos);
    let res = hasher.finalize();
    hex::encode(res)
}

/// Generación de password segura (CSPRNG) sin sesgos.
pub fn generar_password_segura(longitud: usize) -> String {
    let alphabet = "abcdefghjkmnpqrstuvwxyzABCDEFGHJKLMNPQRSTUVWXYZ23456789!@#$%^&*()";
    let alphabet_bytes = alphabet.as_bytes();
    let mut rng = OsRng;
    
    (0..longitud)
        .map(|_| {
            let idx = rng.gen_range(0..alphabet_bytes.len());
            alphabet_bytes[idx] as char
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_derivar_masterkey_consistencia() {
        let password = "password123";
        let salt = b"un-salt-de-16bytes";
        
        let key1 = derivar_masterkey(password, salt).unwrap();
        let key2 = derivar_masterkey(password, salt).unwrap();
        
        assert_eq!(key1, key2);
        assert_eq!(key1.len(), KEY_LEN);
    }

    #[test]
    fn test_derivar_masterkey_diferente() {
        let salt = b"un-salt-de-16bytes";
        let key1 = derivar_masterkey("pass1", salt).unwrap();
        let key2 = derivar_masterkey("pass2", salt).unwrap();
        
        assert_ne!(key1, key2);
    }

    #[test]
    fn test_sellar_y_abrir_dato() {
        let llave = vec![0u8; 32];
        let original = b"mensaje secreto";
        
        let sellado = sellar_dato(original, &llave).unwrap();
        let abierto = abrir_dato(&sellado, &llave).unwrap();
        
        assert_eq!(original, &abierto[..]);
    }

    #[test]
    fn test_sellar_y_abrir_dato_json() {
        let llave = vec![0u8; 32];
        let original = b"datos sensibles json";
        
        let json_blob = sellar_dato_json(original, &llave).unwrap();
        assert!(json_blob.contains("\"nonce\""));
        assert!(json_blob.contains("\"ciphertext\""));
        
        let abierto = abrir_dato_json(&json_blob, &llave).unwrap();
        assert_eq!(original, &abierto[..]);
    }

    #[test]
    fn test_abrir_dato_llave_incorrecta() {
        let llave_correcta = vec![1u8; 32];
        let llave_incorrecta = vec![2u8; 32];
        let original = b"mensaje";
        
        let sellado = sellar_dato(original, &llave_correcta).unwrap();
        let resultado = abrir_dato(&sellado, &llave_incorrecta);
        
        assert!(resultado.is_err());
    }

    #[test]
    fn test_abrir_dato_tampered_ciphertext() {
        let llave = vec![0u8; 32];
        let original = b"mensaje secreto";
        let mut sellado = sellar_dato(original, &llave).unwrap();
        
        // El formato es [nonce(12)] + [ciphertext] + [tag(16)]
        // Modificamos un byte del ciphertext (entre el byte 12 y len-16)
        if sellado.len() > 28 {
            sellado[12] ^= 0xFF; 
        }
        
        let resultado = abrir_dato(&sellado, &llave);
        assert!(resultado.is_err(), "Debería fallar si el ciphertext es alterado");
    }

    #[test]
    fn test_abrir_dato_tampered_tag() {
        let llave = vec![0u8; 32];
        let original = b"mensaje secreto";
        let mut sellado = sellar_dato(original, &llave).unwrap();
        
        // Modificamos un byte del tag (los últimos 16 bytes)
        let len = sellado.len();
        sellado[len - 1] ^= 0xFF;
        
        let resultado = abrir_dato(&sellado, &llave);
        assert!(resultado.is_err(), "Debería fallar si el tag es alterado");
    }

    #[test]
    fn test_abrir_dato_tampered_nonce() {
        let llave = vec![0u8; 32];
        let original = b"mensaje secreto";
        let mut sellado = sellar_dato(original, &llave).unwrap();
        
        // Modificamos un byte del nonce (los primeros 12 bytes)
        sellado[0] ^= 0xFF;
        
        let resultado = abrir_dato(&sellado, &llave);
        assert!(resultado.is_err(), "Debería fallar si el nonce es alterado");
    }

    #[test]
    fn test_hash_integridad() {
        let datos = b"hola mundo";
        let h1 = hash_integridad(datos);
        let h2 = hash_integridad(datos);
        
        assert_eq!(h1, h2);
        assert_eq!(h1.len(), 128); // Blake2b 512 bits = 64 bytes = 128 chars hex
    }

    #[test]
    fn test_generar_password_segura() {
        let p1 = generar_password_segura(16);
        let p2 = generar_password_segura(16);
        
        assert_eq!(p1.len(), 16);
        assert_ne!(p1, p2); // Extremadamente improbable que sean iguales
    }
}
