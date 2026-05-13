pub mod crypto;

use wasm_bindgen::prelude::*;
use crate::crypto::{derivar_masterkey, sellar_dato, abrir_dato};

#[wasm_bindgen]
pub fn wasm_derivar_masterkey(password: &str, salt: &[u8]) -> Result<Vec<u8>, JsValue> {
    derivar_masterkey(password, salt)
        .map_err(|e| JsValue::from_str(&format!("{:?}", e)))
}

#[wasm_bindgen]
pub struct DerivedKeys {
    #[wasm_bindgen(getter_with_clone)]
    pub master_key: Vec<u8>,
    #[wasm_bindgen(getter_with_clone)]
    pub auth_key: Vec<u8>,
}

#[wasm_bindgen]
pub fn wasm_derivar_claves_dual(password: &str, salt: &[u8]) -> Result<DerivedKeys, JsValue> {
    let (master_key, auth_key) = crate::crypto::derivar_claves_dual(password, salt)
        .map_err(|e| JsValue::from_str(&format!("{:?}", e)))?;
    
    Ok(DerivedKeys { master_key, auth_key })
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
}
