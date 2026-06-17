use blake2::{Blake2b512, Digest};
use zeroize::Zeroizing;

fn derive_sync_key(machine_id: &str) -> Zeroizing<Vec<u8>> {
    if let Ok(env_key) = std::env::var("PACT_SYNC_KEY") {
        let key = hex::decode(&env_key).unwrap_or_else(|_| {
            let mut hasher = Blake2b512::new();
            hasher.update(env_key.as_bytes());
            hasher.finalize()[..32].to_vec()
        });
        return Zeroizing::new(if key.len() >= 32 { key[..32].to_vec() } else { key });
    }

    let mut hasher = Blake2b512::new();
    hasher.update(b"pact_sync_v1_encryption_key");
    hasher.update(machine_id.as_bytes());
    let result = hasher.finalize();
    Zeroizing::new(result[..32].to_vec())
}

pub struct SyncCrypto {
    key: Zeroizing<Vec<u8>>,
}

impl SyncCrypto {
    pub fn new(machine_id: &str) -> Self {
        let key = derive_sync_key(machine_id);
        Self { key }
    }

    pub fn encrypt(&self, plaintext: &str) -> String {
        match pact_core::crypto::sellar_dato(plaintext.as_bytes(), &self.key) {
            Ok(raw) => {
                let len = raw.len();
                let blob = pact_core::crypto::EncryptedBlob {
                    nonce: hex::encode(&raw[..12]),
                    tag: hex::encode(&raw[len - 16..]),
                    ciphertext: hex::encode(&raw[12..len - 16]),
                };
                serde_json::to_string(&blob).unwrap_or_else(|_| hex::encode(&raw))
            }
            Err(_) => String::new(),
        }
    }

    pub fn decrypt(&self, encrypted: &str) -> String {
        if encrypted.is_empty() {
            return String::new();
        }

        if encrypted.starts_with('{') {
            match pact_core::crypto::abrir_dato_json(encrypted, &self.key) {
                Ok(bytes) => String::from_utf8(bytes).unwrap_or_default(),
                Err(_) => String::new(),
            }
        } else {
            match hex::decode(encrypted) {
                Ok(raw) => match pact_core::crypto::abrir_dato(&raw, &self.key) {
                    Ok(bytes) => String::from_utf8(bytes).unwrap_or_default(),
                    Err(_) => String::new(),
                },
                Err(_) => String::new(),
            }
        }
    }
}
