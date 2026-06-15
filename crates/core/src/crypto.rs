//! NaCl sealed-box encryption for GitHub Actions secrets. Creating or updating
//! a secret requires encrypting its value with the repository's public key
//! using libsodium's `crypto_box_seal` (an ephemeral X25519 keypair + the
//! recipient's public key, XSalsa20-Poly1305). `crypto_box::seal` is the
//! pure-Rust equivalent; `getrandom`'s `js` feature supplies the browser RNG.

use crypto_box::aead::OsRng;
use crypto_box::PublicKey;

/// Encrypt `plaintext` for GitHub's Actions public key (a base64-encoded raw
/// 32-byte X25519 key), returning the base64 sealed-box ciphertext to send as
/// `encrypted_value` in the `PUT /repos/{o}/{r}/actions/secrets/{name}` body.
pub fn seal_secret(pub_key_b64: &str, plaintext: &str) -> Result<String, String> {
    let key_bytes = crate::github::b64_decode(pub_key_b64)?;
    if key_bytes.len() != 32 {
        return Err(format!("bad public key length: {} (want 32)", key_bytes.len()));
    }
    let mut pk = [0u8; 32];
    pk.copy_from_slice(&key_bytes);
    let public = PublicKey::from(pk);
    let ciphertext = public
        .seal(&mut OsRng, plaintext.as_bytes())
        .map_err(|e| format!("seal: {}", e))?;
    Ok(crate::github::b64_encode(&ciphertext))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The seal output is randomized (ephemeral key) but must always be
    /// non-empty base64 and round-trip-decodable to a sealed-box blob whose
    /// first byte is the ephemeral public key prefix.
    #[test]
    fn seal_produces_sealed_blob() {
        // A throwaway X25519 public key (32 bytes), base64-encoded.
        let pk_b64 = crate::github::b64_encode(&[0u8; 32]);
        let ct = seal_secret(&pk_b64, "hunter2").unwrap();
        assert!(!ct.is_empty());
        // A sealed box = 32-byte ephemeral pubkey + 16-byte MAC + ciphertext.
        let raw = crate::github::b64_decode(&ct).unwrap();
        assert!(raw.len() > 32 + 16, "sealed blob too short: {}", raw.len());
    }
}
