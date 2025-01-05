use base64::prelude::*;
use std::ffi::OsStr;

use ring::aead::{Aad, LessSafeKey, Nonce, UnboundKey, AES_256_GCM};
use ring::digest::{Context, SHA256};
use ring::rand::{SecureRandom, SystemRandom};

/// Generates a random 256-bit key.
pub fn generate_key() -> [u8; 32] {
    let rng = SystemRandom::new();
    let mut key = [0u8; 32];
    rng.fill(&mut key).expect("Failed to generate key");
    key
}

/// Encrypts data and returns a combined ciphertext (nonce + encrypted data).
pub fn encrypt_data(key: &[u8; 32], plaintext: &[u8]) -> Vec<u8> {
    let rng = SystemRandom::new();
    let mut nonce_bytes = [0u8; 12];
    rng.fill(&mut nonce_bytes)
        .expect("Failed to generate nonce");

    let unbound_key = UnboundKey::new(&AES_256_GCM, key).expect("Failed to create encryption key");
    let key = LessSafeKey::new(unbound_key);
    let nonce = Nonce::assume_unique_for_key(nonce_bytes);

    let mut in_out = plaintext.to_vec();
    key.seal_in_place_append_tag(nonce, Aad::empty(), &mut in_out)
        .expect("Encryption failed");

    // Combine nonce and ciphertext
    let mut combined = nonce_bytes.to_vec();
    combined.extend_from_slice(&in_out);
    combined
}

/// Decrypts a combined ciphertext (nonce + encrypted data).
pub fn decrypt_data(key: &[u8; 32], combined: &[u8]) -> Vec<u8> {
    // Extract the nonce (first 12 bytes) and ciphertext
    let (nonce_bytes, ciphertext) = combined.split_at(12);
    let nonce = Nonce::assume_unique_for_key(nonce_bytes.try_into().expect("Invalid nonce length"));

    let unbound_key = UnboundKey::new(&AES_256_GCM, key).expect("Failed to create decryption key");
    let key = LessSafeKey::new(unbound_key);

    let mut in_out = ciphertext.to_vec();
    let plaintext = key
        .open_in_place(nonce, Aad::empty(), &mut in_out)
        .expect("Decryption failed");
    plaintext.to_vec()
}

/// Scrambles a file name into a POSIX-compliant Base64-encoded string using SHA-256 hash.
/// Returns a Base64-encoded string, ensuring no illegal characters.
pub fn hash_file_name(file_name: &OsStr) -> String {
    let mut context = Context::new(&SHA256);
    context.update(file_name.as_encoded_bytes());
    let digest = context.finish();
    let mut result = BASE64_STANDARD.encode(digest.as_ref());
    result.truncate(255);
    result
}
