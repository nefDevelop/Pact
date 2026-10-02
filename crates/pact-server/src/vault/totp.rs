use totp_rs::{Secret, Algorithm, TOTP};

/// Normaliza el seed TOTP almacenado para que `totp-rs` pueda decodificarlo.
///
/// Acepta el seed en Base32 (formato estándar de las apps de autenticación,
/// p. ej. `JBSWY3DPEHPK3PXP`) y también URIs `otpauth://` completas, de las
/// que extrae únicamente el parámetro `secret`. El Base32 se normaliza a
/// mayúsculas, sin espacios ni relleno `=`.
fn normalize_seed(raw: &str) -> String {
    let trimmed = raw.trim();

    if trimmed.to_ascii_lowercase().starts_with("otpauth://") {
        if let Ok(uri) = url::Url::parse(trimmed) {
            if let Some((_, value)) = uri.query_pairs().find(|(key, _)| key == "secret") {
                return value
                    .trim()
                    .to_ascii_uppercase()
                    .replace(' ', "")
                    .trim_end_matches('=')
                    .to_string();
            }
        }
    }

    trimmed
        .to_ascii_uppercase()
        .replace(' ', "")
        .trim_end_matches('=')
        .to_string()
}

fn generate_totp_code(seed: &str) -> Result<String, String> {
    let normalized = normalize_seed(seed);
    let secret = Secret::Encoded(normalized)
        .to_bytes()
        .map_err(|e| format!("Secret error: {e:?}"))?;

    // `new_unchecked` evita la validación de tamaño mínimo (128 bits) de
    // `TOTP::new`. Los seeds de 80 bits (16 caracteres Base32) son el estándar
    // de facto de la mayoría de autenticadores y su HMAC es válido.
    let totp = TOTP::new_unchecked(
        Algorithm::SHA1,
        6,
        1,
        30,
        secret,
        None,
        String::new(),
    );

    totp.generate_current()
        .map_err(|e| format!("TOTP generate error: {e:?}"))
}

pub fn get_totp_code(seed: &str) -> String {
    generate_totp_code(seed).unwrap_or_else(|_| "------".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generates_code_from_base32_seed() {
        // RFC 6238 test vector: seed "12345678901234567890" en Base32.
        let code = get_totp_code("GEZDGNBVGY3TQOJQGEZDGNBVGY3TQOJQ");
        assert_ne!(code, "------");
        assert_eq!(code.len(), 6);
    }

    #[test]
    fn generates_code_from_short_80_bit_seed() {
        // Seed típico de 10 bytes (16 caracteres Base32) usado por la mayoría
        // de autenticadores; `TOTP::new` lo rechazaría por tamaño.
        let code = get_totp_code("JBSWY3DPEHPK3PXP");
        assert_eq!(code.len(), 6);
        assert!(code.chars().all(|c| c.is_ascii_digit()), "código inesperado: {code:?}");
    }

    #[test]
    fn normalizes_lowercase_and_spaces() {
        assert_eq!(normalize_seed(" jbswy3dp ehpk3pxp "), "JBSWY3DPEHPK3PXP");
    }

    #[test]
    fn extracts_secret_from_otpauth_uri() {
        let uri = "otpauth://totp/Test:me?secret=JBSWY3DPEHPK3PXP&issuer=Test";
        assert_eq!(normalize_seed(uri), "JBSWY3DPEHPK3PXP");
    }
}