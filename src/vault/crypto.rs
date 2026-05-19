use aes_gcm::{
    aead::{Aead, KeyInit},
    Aes256Gcm, Key, Nonce,
};
use anyhow::{Context, Result};
use base64::{engine::general_purpose::STANDARD as B64, Engine};
use rand::RngCore;
use scrypt::{scrypt, Params};

pub const KEY_LEN: usize = 32;
pub const SALT_LEN: usize = 32;
pub const NONCE_LEN: usize = 12;

/// Derives a 256-bit key using scrypt (N=2^15, r=8, p=1).
pub fn derive_key(passphrase: &str, salt: &[u8]) -> Result<[u8; KEY_LEN]> {
    let params = Params::new(15, 8, 1, KEY_LEN)?;
    let mut key = [0u8; KEY_LEN];
    scrypt(passphrase.as_bytes(), salt, &params, &mut key)
        .context("scrypt key derivation failed")?;
    Ok(key)
}

pub fn gen_salt() -> [u8; SALT_LEN] {
    let mut buf = [0u8; SALT_LEN];
    rand::thread_rng().fill_bytes(&mut buf);
    buf
}

pub fn gen_nonce() -> [u8; NONCE_LEN] {
    let mut buf = [0u8; NONCE_LEN];
    rand::thread_rng().fill_bytes(&mut buf);
    buf
}

/// Encrypts plaintext with AES-256-GCM; returns (ciphertext, nonce).
pub fn encrypt(key: &[u8; KEY_LEN], plaintext: &[u8]) -> Result<(Vec<u8>, [u8; NONCE_LEN])> {
    let nonce_bytes = gen_nonce();
    let cipher = Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(key));
    let nonce = Nonce::from_slice(&nonce_bytes);
    let ciphertext = cipher
        .encrypt(nonce, plaintext)
        .map_err(|e| anyhow::anyhow!("AES-GCM encrypt: {e}"))?;
    Ok((ciphertext, nonce_bytes))
}

/// Decrypts ciphertext with AES-256-GCM.
pub fn decrypt(key: &[u8; KEY_LEN], nonce_bytes: &[u8], ciphertext: &[u8]) -> Result<Vec<u8>> {
    let cipher = Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(key));
    let nonce = Nonce::from_slice(nonce_bytes);
    cipher
        .decrypt(nonce, ciphertext)
        .map_err(|_| anyhow::anyhow!("Decryption failed — wrong passphrase or corrupted vault"))
}

pub fn b64_encode(data: &[u8]) -> String { B64.encode(data) }

pub fn b64_decode(s: &str) -> Result<Vec<u8>> {
    B64.decode(s).context("invalid base64")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_derive_key() {
        let passphrase = "password123";
        let salt = gen_salt();
        let key = derive_key(passphrase, &salt).unwrap();
        assert_eq!(key.len(), KEY_LEN);

        let key2 = derive_key(passphrase, &salt).unwrap();
        assert_eq!(key, key2);

        let key3 = derive_key("different", &salt).unwrap();
        assert_ne!(key, key3);
    }

    #[test]
    fn test_encrypt_decrypt() {
        let passphrase = "secure_journal";
        let salt = gen_salt();
        let key = derive_key(passphrase, &salt).unwrap();
        let plaintext = b"Hello, this is a secret entry.";

        let (ciphertext, nonce) = encrypt(&key, plaintext).unwrap();
        assert_ne!(plaintext.to_vec(), ciphertext);

        let decrypted = decrypt(&key, &nonce, &ciphertext).unwrap();
        assert_eq!(plaintext.to_vec(), decrypted);
    }

    #[test]
    fn test_decrypt_failure() {
        let passphrase = "secure_journal";
        let salt = gen_salt();
        let key = derive_key(passphrase, &salt).unwrap();
        let plaintext = b"Secret data";

        let (ciphertext, nonce) = encrypt(&key, plaintext).unwrap();

        let wrong_key = [0u8; KEY_LEN];
        let result = decrypt(&wrong_key, &nonce, &ciphertext);
        assert!(result.is_err());
    }

    #[test]
    fn test_base64() {
        let data = b"some data to encode";
        let encoded = b64_encode(data);
        let decoded = b64_decode(&encoded).unwrap();
        assert_eq!(data.to_vec(), decoded);
    }
}
