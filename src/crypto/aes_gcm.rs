use aes_gcm::{
    aead::{Aead, KeyInit},
    Aes256Gcm, Nonce,
};
use anyhow::{Context, Result};
use base64::{engine::general_purpose::STANDARD, Engine as _};
use std::fs;

/// Decrypts AES-GCM encrypted data using a key from a file
pub fn decrypt_from_key_file(key_path: &str, cipher_text: &str) -> Result<String> {
    let key_data = fs::read_to_string(key_path)
        .with_context(|| format!("Failed to read key file: {}", key_path))?;

    let key = STANDARD
        .decode(key_data.trim())
        .context("Failed to decode base64 key")?;

    decrypt_with_key(&key, cipher_text)
}

/// Decrypts AES-GCM encrypted data using a base64-encoded key string
pub fn decrypt_from_base64_key(private_key: &str, cipher_text: &str) -> Result<String> {
    let key = STANDARD
        .decode(private_key.trim())
        .context("Failed to decode base64 private key")?;

    decrypt_with_key(&key, cipher_text)
}

/// Core decryption function - pure function that takes key bytes and cipher text
fn decrypt_with_key(key: &[u8], cipher_text: &str) -> Result<String> {
    let encrypted_data = STANDARD
        .decode(cipher_text)
        .context("Failed to decode base64 cipher text")?;

    if encrypted_data.len() < 28 {
        anyhow::bail!("Encrypted data too short (need at least 28 bytes for nonce + tag)");
    }

    // Extract components: first 12 bytes (nonce), last 16 bytes (tag), middle (ciphertext)
    let (nonce_bytes, rest) = encrypted_data.split_at(12);
    let (ciphertext, tag) = rest.split_at(rest.len() - 16);

    // Create cipher and decrypt
    let cipher = Aes256Gcm::new_from_slice(key).context("Invalid key length for AES-256-GCM")?;

    let nonce = Nonce::from_slice(nonce_bytes);

    // Combine ciphertext and tag for decryption
    let mut payload = Vec::with_capacity(ciphertext.len() + tag.len());
    payload.extend_from_slice(ciphertext);
    payload.extend_from_slice(tag);

    let plaintext = cipher
        .decrypt(nonce, payload.as_slice())
        .map_err(|e| anyhow::anyhow!("Decryption failed: {:?}", e))?;

    String::from_utf8(plaintext).context("Decrypted data is not valid UTF-8")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_decrypt_with_key() {
        // This would need actual test vectors in a real implementation
        // For now, just test the function signature and error handling
        let key = vec![0u8; 32]; // 256-bit key
        let result = decrypt_with_key(&key, "invalid_base64");
        assert!(result.is_err());
    }
}
