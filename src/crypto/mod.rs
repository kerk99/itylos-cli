use aes_gcm::{
    Aes256Gcm, Nonce,
    aead::{Aead, KeyInit},
};
use anyhow::{Context, Result, bail};
use base64::{
    Engine as _,
    engine::general_purpose::{STANDARD, URL_SAFE_NO_PAD},
};
use ed25519_dalek::{Signature, Verifier, VerifyingKey};
use hmac::Hmac;
use pbkdf2::pbkdf2;
use rand::RngCore;
use serde_json::{Map, Value};
use sha2::{Digest, Sha256};
use zeroize::{Zeroize, Zeroizing};

use crate::error::ItylosError;
use crate::types::{AadV2, PaddedPayload, SERVER_PUB_KEY_B64};

pub struct EncryptionOutcome {
    pub payload: String,
    pub aad_hash_hex: String,
    pub key_fragment: String,
    pub salt_b64: Option<String>,
}

pub fn generate_url_key() -> Result<Zeroizing<Vec<u8>>> {
    let mut key = Zeroizing::new(vec![0u8; 32]);
    rand::rngs::OsRng.fill_bytes(&mut key);
    Ok(key)
}

pub fn encode_url_key(url_key: &[u8]) -> String {
    URL_SAFE_NO_PAD.encode(url_key)
}

pub fn derive_key(url_key: &[u8]) -> Zeroizing<Vec<u8>> {
    let digest = Sha256::digest(url_key);
    Zeroizing::new(digest.to_vec())
}

/// Derive a 256-bit key from a password + salt using PBKDF2-HMAC-SHA256 (300k iterations).
/// Compatible with the JS frontend (crypto.js derivePasswordKey fallback).
pub fn derive_password_key(password: &str, salt: &[u8]) -> Result<Zeroizing<Vec<u8>>> {
    let mut derived = Zeroizing::new(vec![0u8; 32]);
    pbkdf2::<Hmac<Sha256>>(password.as_bytes(), salt, 300_000, &mut derived)
        .map_err(|_| ItylosError::Message("erreur PBKDF2".to_string()))?;
    Ok(derived)
}

/// Derive final AES key from url_key + password: SHA-256(url_key || pwd_key).
/// Compatible with the JS frontend (crypto.js encryptSecret with password).
pub fn derive_combined_key(
    url_key: &[u8],
    password: &str,
    salt: &[u8],
) -> Result<Zeroizing<Vec<u8>>> {
    let pwd_key = derive_password_key(password, salt)?;
    let mut combined = Vec::with_capacity(url_key.len() + pwd_key.len());
    combined.extend_from_slice(url_key);
    combined.extend_from_slice(&pwd_key);
    let digest = Sha256::digest(&combined);
    combined.zeroize();
    Ok(Zeroizing::new(digest.to_vec()))
}

pub fn generate_salt() -> Vec<u8> {
    let mut salt = vec![0u8; 16];
    rand::rngs::OsRng.fill_bytes(&mut salt);
    salt
}

pub fn build_aad(ttl: u64) -> Result<Vec<u8>> {
    serde_json::to_vec(&AadV2 {
        v: "2.0".to_string(),
        alg: "AES-256-GCM".to_string(),
        ttl,
    })
    .context("erreur AAD")
}

pub fn compute_aad_hash_hex(ttl: u64) -> Result<String> {
    let aad = build_aad(ttl)?;
    Ok(hex::encode(Sha256::digest(&aad)))
}

pub fn pad_content(content: &str) -> Result<PaddedPayload> {
    let length = content.len();
    let target = if length < 1024 {
        1024
    } else if length < 10_240 {
        10_240
    } else {
        length + 512
    };

    let mut noise = vec![0u8; target.saturating_sub(length)];
    rand::rngs::OsRng.fill_bytes(&mut noise);
    let noise_b64 = STANDARD.encode(&noise);
    noise.zeroize();

    Ok(PaddedPayload {
        content: content.to_string(),
        noise: noise_b64,
    })
}

pub fn encrypt_local_with_password(
    message_json: &str,
    ttl: u64,
    password: Option<&str>,
) -> Result<EncryptionOutcome> {
    let url_key = generate_url_key()?;
    let key_fragment = encode_url_key(&url_key);
    let payload = pad_content(message_json)?;
    let plaintext_json = serde_json::to_vec(&payload).context("erreur de serialisation JSON")?;
    let mut plaintext = Zeroizing::new(plaintext_json);

    let (final_key, salt) = if let Some(pwd) = password {
        let salt = generate_salt();
        let key = derive_combined_key(&url_key, pwd, &salt)?;
        (key, Some(salt))
    } else {
        (derive_key(&url_key), None)
    };

    let cipher = Aes256Gcm::new_from_slice(&final_key).context("erreur AES")?;
    let mut nonce_bytes = [0u8; 12];
    rand::rngs::OsRng.fill_bytes(&mut nonce_bytes);
    let nonce = Nonce::from_slice(&nonce_bytes);
    let aad_bytes = build_aad(ttl)?;
    let sealed = cipher
        .encrypt(
            nonce,
            aes_gcm::aead::Payload {
                msg: &plaintext,
                aad: &aad_bytes,
            },
        )
        .map_err(|_| ItylosError::Message("erreur GCM".to_string()))?;

    plaintext.zeroize();

    Ok(EncryptionOutcome {
        payload: format!(
            "{}.{}",
            URL_SAFE_NO_PAD.encode(sealed),
            URL_SAFE_NO_PAD.encode(nonce_bytes)
        ),
        aad_hash_hex: compute_aad_hash_hex(ttl)?,
        key_fragment,
        salt_b64: salt.map(|s| URL_SAFE_NO_PAD.encode(s)),
    })
}

pub fn decrypt_local(payload_str: &str, key_b64: &str, ttl: u64) -> Result<Zeroizing<String>> {
    decrypt_local_with_password(payload_str, key_b64, ttl, None, None)
}

pub fn decrypt_local_with_password(
    payload_str: &str,
    key_b64: &str,
    ttl: u64,
    password: Option<&str>,
    salt_b64: Option<&str>,
) -> Result<Zeroizing<String>> {
    let url_key = URL_SAFE_NO_PAD
        .decode(key_b64)
        .context("cle URL invalide")?;
    let (sealed_b64, nonce_b64) = payload_str
        .split_once('.')
        .ok_or_else(|| ItylosError::Message("format de payload invalide".to_string()))?;

    let sealed = URL_SAFE_NO_PAD
        .decode(sealed_b64)
        .context("ciphertext invalide")?;
    let nonce_bytes = URL_SAFE_NO_PAD
        .decode(nonce_b64)
        .context("nonce invalide")?;
    if nonce_bytes.len() != 12 {
        bail!("nonce invalide");
    }

    let final_key = match (password, salt_b64) {
        (Some(pwd), Some(salt)) => {
            let salt_bytes = URL_SAFE_NO_PAD.decode(salt).context("salt invalide")?;
            derive_combined_key(&url_key, pwd, &salt_bytes)?
        }
        _ => derive_key(&url_key),
    };

    let cipher = Aes256Gcm::new_from_slice(&final_key).context("erreur AES")?;
    let aad_bytes = build_aad(ttl)?;
    let plaintext = cipher
        .decrypt(
            Nonce::from_slice(&nonce_bytes),
            aes_gcm::aead::Payload {
                msg: sealed.as_ref(),
                aad: &aad_bytes,
            },
        )
        .map_err(|_| {
            ItylosError::Message(
                "dechiffrement echoue (cle invalide, mot de passe incorrect ou donnee corrompue)"
                    .to_string(),
            )
        })?;

    let mut plaintext = Zeroizing::new(plaintext);
    let padded: PaddedPayload =
        serde_json::from_slice(&plaintext).context("donnee corrompue ou format JSON invalide")?;
    plaintext.zeroize();

    Ok(Zeroizing::new(padded.content))
}

pub fn verify_proof_signature(proof: &Value) -> Result<bool> {
    let mut unsigned = canonicalize_json(proof);
    let verification = unsigned
        .get_mut("verification")
        .and_then(Value::as_object_mut)
        .ok_or_else(|| {
            ItylosError::Message(
                "Preuve malformee : bloc 'verification' absent ou invalide.".to_string(),
            )
        })?;

    let signature_b64 = verification
        .get("ed25519_signature")
        .and_then(Value::as_str)
        .ok_or(ItylosError::UnsignedProof)?
        .to_string();
    if signature_b64.is_empty() || signature_b64 == "unsigned" {
        return Err(ItylosError::UnsignedProof.into());
    }

    verification.insert(
        "ed25519_signature".to_string(),
        Value::String(String::new()),
    );

    let payload =
        serde_json::to_vec(&unsigned).context("erreur de re-serialisation de la preuve")?;
    let signature_bytes = STANDARD
        .decode(signature_b64)
        .context("signature base64 invalide")?;
    let public_key_bytes = STANDARD
        .decode(SERVER_PUB_KEY_B64)
        .context("cle publique serveur invalide")?;
    let public_key_array: [u8; 32] = public_key_bytes.try_into().map_err(|_| {
        ItylosError::Message("cle publique Ed25519 de taille incorrecte.".to_string())
    })?;

    let public_key =
        VerifyingKey::from_bytes(&public_key_array).context("cle publique Ed25519 invalide")?;
    let signature =
        Signature::from_slice(&signature_bytes).context("signature Ed25519 invalide")?;

    Ok(public_key.verify(&payload, &signature).is_ok())
}

fn canonicalize_json(value: &Value) -> Value {
    match value {
        Value::Object(map) => {
            let mut entries: Vec<_> = map.iter().collect();
            entries.sort_by(|a, b| a.0.cmp(b.0));
            let mut out = Map::new();
            for (key, child) in entries {
                out.insert(key.clone(), canonicalize_json(child));
            }
            Value::Object(out)
        }
        Value::Array(values) => Value::Array(values.iter().map(canonicalize_json).collect()),
        _ => value.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn aad_matches_expected_json_shape() {
        let aad = build_aad(3600).expect("aad should build");
        assert_eq!(
            String::from_utf8(aad).expect("utf8"),
            r#"{"v":"2.0","alg":"AES-256-GCM","ttl":3600}"#
        );
    }

    #[test]
    fn padding_uses_expected_buckets() {
        let short = pad_content("abc").expect("padding should work");
        assert!(short.noise.len() > 1000);

        let medium = pad_content(&"a".repeat(2000)).expect("padding should work");
        assert!(medium.noise.len() > 10_000);

        let long = pad_content(&"a".repeat(11_000)).expect("padding should work");
        assert!(!long.noise.is_empty());
    }

    #[test]
    fn encrypt_then_decrypt_roundtrip() {
        let message =
            r#"{"protocol":"ITYLOS_CAPSULE_V3_MULTI","message":"bonjour","attachments":[]}"#;
        let encrypted =
            encrypt_local_with_password(message, 3600, None).expect("encryption should work");
        let decrypted =
            decrypt_local(&encrypted.payload, &encrypted.key_fragment, 3600).expect("decrypt");
        assert_eq!(&*decrypted, message);
        assert_eq!(encrypted.aad_hash_hex.len(), 64);
    }

    #[test]
    fn aad_hash_matches_contract_value() {
        let aad_hash = compute_aad_hash_hex(3600).expect("aad hash");
        assert_eq!(
            aad_hash,
            "09f7754b7ff9a179b9eccf1607134557d157c9f6722ab8817c29c20fc2343fcd"
        );
    }

    #[test]
    fn encrypt_then_decrypt_roundtrip_with_password() {
        let message =
            r#"{"protocol":"ITYLOS_CAPSULE_V3_MULTI","message":"confidentiel","attachments":[]}"#;
        let encrypted = encrypt_local_with_password(message, 3600, Some("hunter2"))
            .expect("encryption with password should work");
        assert!(encrypted.salt_b64.is_some());

        // Decrypt with correct password
        let decrypted = decrypt_local_with_password(
            &encrypted.payload,
            &encrypted.key_fragment,
            3600,
            Some("hunter2"),
            encrypted.salt_b64.as_deref(),
        )
        .expect("decrypt with correct password");
        assert_eq!(&*decrypted, message);

        // Decrypt with wrong password should fail
        let error = decrypt_local_with_password(
            &encrypted.payload,
            &encrypted.key_fragment,
            3600,
            Some("wrong"),
            encrypted.salt_b64.as_deref(),
        )
        .expect_err("wrong password should fail");
        assert!(error.to_string().contains("dechiffrement echoue"));

        // Decrypt without password should fail
        let error2 = decrypt_local(&encrypted.payload, &encrypted.key_fragment, 3600)
            .expect_err("missing password should fail");
        assert!(error2.to_string().contains("dechiffrement echoue"));
    }

    #[test]
    fn decrypt_rejects_wrong_ttl() {
        let message =
            r#"{"protocol":"ITYLOS_CAPSULE_V3_MULTI","message":"bonjour","attachments":[]}"#;
        let encrypted =
            encrypt_local_with_password(message, 3600, None).expect("encryption should work");
        let error = decrypt_local(&encrypted.payload, &encrypted.key_fragment, 86_400)
            .expect_err("ttl mismatch should fail");
        assert!(error.to_string().contains("dechiffrement echoue"));
    }

    #[test]
    fn decrypt_rejects_invalid_payload_shape() {
        let valid_key = encode_url_key(&[0u8; 32]);
        let error =
            decrypt_local("abc", &valid_key, 3600).expect_err("invalid payload should fail");
        assert!(error.to_string().contains("format de payload invalide"));
    }

    #[test]
    fn verify_proof_rejects_unsigned_and_invalid_proofs() {
        let unsigned = json!({
            "verification": {
                "ed25519_signature": "unsigned"
            }
        });
        let error = verify_proof_signature(&unsigned).expect_err("unsigned proof should fail");
        assert!(error.to_string().contains("n'est pas signe"));

        let malformed = json!({});
        let error = verify_proof_signature(&malformed).expect_err("malformed proof should fail");
        assert!(
            error
                .to_string()
                .contains("bloc 'verification' absent ou invalide")
        );
    }

    #[test]
    fn canonicalization_sorts_object_keys_recursively() {
        let value = json!({
            "z": 1,
            "a": {
                "b": 2,
                "a": 1
            }
        });

        let out = canonicalize_json(&value);
        let serialized = serde_json::to_string(&out).expect("serialize");
        assert_eq!(serialized, r#"{"a":{"a":1,"b":2},"z":1}"#);
    }
}
