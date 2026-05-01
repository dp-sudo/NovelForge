//! AES-256-GCM encryption/decryption for local secret file fallback.
//!
//! Only used when Windows Credential Manager is unavailable.
//! Key is derived from machine-specific data combined with a static salt.

use aes_gcm::{
    aead::{Aead, KeyInit},
    Aes256Gcm, Nonce,
};
use sha2::{Digest, Sha256};
use uuid::Uuid;

use crate::errors::AppErrorDto;

const SALT: &[u8] = b"novelforge-v0.1-aes-key-salt";

/// Derive a 256-bit key from machine context.
fn derive_key() -> Vec<u8> {
    let mut hasher = Sha256::new();
    hasher.update(SALT);
    if let Some(info) = hostname_info() {
        hasher.update(info);
    }
    hasher.update(env!("CARGO_PKG_VERSION"));
    hasher.finalize().to_vec()
}

fn hostname_info() -> Option<String> {
    std::env::var("COMPUTERNAME")
        .ok()
        .or_else(|| std::env::var("HOSTNAME").ok())
        .or_else(|| {
            std::fs::read_to_string("/etc/hostname")
                .ok()
                .map(|s| s.trim().to_string())
        })
}

/// Encrypt plaintext with AES-256-GCM. Returns base64-encoded (nonce || ciphertext).
pub fn encrypt(plaintext: &str) -> Result<String, AppErrorDto> {
    let key = derive_key();
    let cipher = Aes256Gcm::new_from_slice(&key).map_err(|e| {
        AppErrorDto::new("CRYPTO_INIT_FAILED", "无法初始化加密器", false)
            .with_detail(e.to_string())
    })?;

    let mut nonce_bytes = [0u8; 12];
    nonce_bytes.copy_from_slice(&Uuid::new_v4().as_bytes()[..12]);
    let nonce = Nonce::from_slice(&nonce_bytes);

    let ciphertext = cipher.encrypt(nonce, plaintext.as_bytes()).map_err(|e| {
        AppErrorDto::new("CRYPTO_ENCRYPT_FAILED", "加密失败", false)
            .with_detail(e.to_string())
    })?;

    let mut result = nonce_bytes.to_vec();
    result.extend(ciphertext);
    Ok(base64_encode(&result))
}

/// Decrypt base64-encoded (nonce || ciphertext) with AES-256-GCM.
pub fn decrypt(encoded: &str) -> Result<String, AppErrorDto> {
    let data = base64_decode(encoded)
        .map_err(|_| AppErrorDto::new("CRYPTO_DECODE_FAILED", "Base64 数据无效", false))?;

    if data.len() < 12 {
        return Err(AppErrorDto::new(
            "CRYPTO_INVALID_DATA",
            "加密数据长度不足",
            false,
        ));
    }

    let (nonce_bytes, ciphertext) = data.split_at(12);
    let nonce = Nonce::from_slice(nonce_bytes);

    let key = derive_key();
    let cipher = Aes256Gcm::new_from_slice(&key).map_err(|e| {
        AppErrorDto::new("CRYPTO_INIT_FAILED", "无法初始化解密器", false)
            .with_detail(e.to_string())
    })?;

    let plaintext = cipher.decrypt(nonce, ciphertext).map_err(|_| {
        AppErrorDto::new(
            "CRYPTO_DECRYPT_FAILED",
            "解密失败：密钥不匹配或数据已损坏",
            false,
        )
    })?;

    String::from_utf8(plaintext).map_err(|_| {
        AppErrorDto::new(
            "CRYPTO_INVALID_UTF8",
            "解密结果不是有效的 UTF-8 文本",
            false,
        )
    })
}

fn base64_encode(data: &[u8]) -> String {
    use base64::{engine::general_purpose::STANDARD, Engine as _};
    STANDARD.encode(data)
}

fn base64_decode(data: &str) -> Result<Vec<u8>, String> {
    use base64::{engine::general_purpose::STANDARD, Engine as _};
    STANDARD.decode(data).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::{decrypt, encrypt};

    #[test]
    fn encrypt_and_decrypt_roundtrip_succeeds() {
        let plaintext = "novelforge-secret-token";
        let encoded = encrypt(plaintext).expect("encrypt should succeed");
        let decoded = decrypt(&encoded).expect("decrypt should succeed");
        assert_eq!(decoded, plaintext);
    }

    #[test]
    fn decrypt_rejects_invalid_base64() {
        let err = decrypt("%%%").expect_err("invalid base64 should fail");
        assert_eq!(err.code, "CRYPTO_DECODE_FAILED");
    }

    #[test]
    fn decrypt_rejects_too_short_payload() {
        let err = decrypt("YQ==").expect_err("too short payload should fail");
        assert_eq!(err.code, "CRYPTO_INVALID_DATA");
    }
}
