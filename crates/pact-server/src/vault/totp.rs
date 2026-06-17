use totp_rs::{Secret, Algorithm, TOTP};

fn generate_totp_code(secret_hex: &str) -> Result<String, String> {
    let secret_bytes = hex::decode(secret_hex).map_err(|e| format!("Hex error: {}", e))?;
    let secret = Secret::Raw(secret_bytes)
        .to_bytes()
        .map_err(|e| format!("Secret error: {:?}", e))?;

    let totp = TOTP::new(
        Algorithm::SHA1,
        6,
        1,
        30,
        secret,
        None,
        String::new(),
    ).map_err(|e| format!("TOTP init error: {:?}", e))?;

    totp.generate_current()
        .map_err(|e| format!("TOTP generate error: {:?}", e))
}

pub fn get_totp_code(secret_hex: &str) -> String {
    generate_totp_code(secret_hex).unwrap_or_else(|_| "------".to_string())
}
