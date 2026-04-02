use anyhow::{Context, Result, bail};
use base64::{
    Engine as _,
    engine::general_purpose::{STANDARD, URL_SAFE_NO_PAD},
};
use regex::Regex;
use serde_json::Value;
use std::path::{Path, PathBuf};
use zeroize::Zeroize;

use crate::{
    crypto,
    error::ItylosError,
    network::ItylosApi,
    types::{
        BurnProofEnvelope, CAPSULE_PROTOCOL, CapsuleFileV3, CapsuleV3, CreateReq, DOMAIN, FetchRes,
        MAX_ATTACHMENT_BYTES, SendOptions, Ttl, normalize_ttl,
    },
    ui,
};

pub fn send_secret(api: &ItylosApi, options: SendOptions) -> Result<()> {
    let link = create_capsule_link(
        api,
        options.text,
        options.file.clone(),
        options.ttl,
        options.password.as_deref(),
    )?;
    ui::print_link(&link);
    if options.password.is_some() {
        ui::print_password_reminder();
    }
    Ok(())
}

pub fn create_capsule_link(
    api: &ItylosApi,
    text: String,
    file: Option<PathBuf>,
    ttl: Ttl,
    password: Option<&str>,
) -> Result<String> {
    let capsule = build_capsule(text, file)?;
    let capsule_json = serde_json::to_string(&capsule).context("Erreur de serialisation")?;
    let encrypted = crypto::encrypt_local_with_password(&capsule_json, ttl.seconds(), password)?;
    let has_password = encrypted.salt_b64.is_some();
    validate_create_request_parts(
        &encrypted.payload,
        ttl.seconds(),
        &encrypted.aad_hash_hex,
        has_password,
        encrypted.salt_b64.as_deref(),
    )?;

    let response = api.create_secret(&CreateReq {
        payload: encrypted.payload,
        ttl: normalize_ttl(ttl.seconds()),
        aad_hash: encrypted.aad_hash_hex,
        has_password,
        pwd_salt: encrypted.salt_b64,
    })?;

    if !response.success {
        bail!(response.error.unwrap_or_else(|| "Erreur API".to_string()));
    }

    Ok(format!(
        "{}/v/{}#{}",
        DOMAIN, response.secret_id, encrypted.key_fragment
    ))
}

pub fn read_secret(api: &ItylosApi, url: &str) -> Result<()> {
    let (secret_id, key_fragment) = parse_secret_url(url)?;
    let response = api.fetch_secret(&secret_id)?;
    validate_fetch_response_pre_password(&response)?;

    let payload = response
        .payload
        .as_deref()
        .ok_or_else(|| ItylosError::Message("payload absent".to_string()))?;

    let decrypted = if response.has_password {
        let password = ui::prompt_password()?;
        crypto::decrypt_local_with_password(
            payload,
            &key_fragment,
            response.ttl,
            Some(&password),
            response.pwd_salt.as_deref(),
        )?
    } else {
        crypto::decrypt_local(payload, &key_fragment, response.ttl)?
    };

    ui::print_decrypted_header();
    render_capsule(&decrypted)?;
    ui::print_decrypted_footer();

    let burn = api.burn_secret_with_proof(&secret_id)?;
    if burn.success {
        if let Some(ref proof) = burn.proof {
            let normalized = normalize_proof_document(serde_json::json!({ "proof": proof }))?;
            let signature_ok = crypto::verify_proof_signature(&normalized).unwrap_or(false);
            if signature_ok {
                ui::print_burn_verified();
            } else {
                ui::print_burn_unverified();
            }
            ui::print_proof_id_hint(&proof.proof_id);
        } else {
            ui::print_burn_no_proof();
        }
    } else {
        ui::print_burn_failed();
    }
    Ok(())
}

pub fn verify_proof(input: &str, api: Option<&ItylosApi>) -> Result<()> {
    let hex32 = Regex::new(r"^[a-fA-F0-9]{32}$").expect("valid regex");
    let json = if hex32.is_match(input) {
        let api = api.ok_or_else(|| {
            ItylosError::Message("Connexion necessaire pour verifier un proof_id.".to_string())
        })?;
        ui::print_fetching_proof(input);
        let mut proof = api.fetch_proof(input)?;
        // Remove fields added by proof_download that weren't in the signed payload
        if let Some(obj) = proof.as_object_mut() {
            obj.remove("download");
            if let Some(v) = obj.get_mut("verification").and_then(|v| v.as_object_mut()) {
                v.remove("ed25519_public_key");
            }
        }
        proof
    } else {
        let path = Path::new(input);
        let payload = std::fs::read_to_string(path)
            .with_context(|| format!("Fichier introuvable : {}", path.display()))?;
        serde_json::from_str(&payload).context("JSON invalide")?
    };

    if crypto::verify_proof_signature(&json)? {
        ui::print_proof_authentic();
    } else {
        ui::print_proof_forged();
    }
    Ok(())
}

fn build_capsule(text: String, file: Option<PathBuf>) -> Result<CapsuleV3> {
    let mut capsule = CapsuleV3 {
        protocol: CAPSULE_PROTOCOL.to_string(),
        message: String::new(),
        attachments: Vec::new(),
    };

    if let Some(file_path) = file {
        let file_bytes = std::fs::read(&file_path).context("Erreur de lecture du fichier")?;
        let file_size = file_bytes.len();
        if file_bytes.len() > MAX_ATTACHMENT_BYTES {
            return Err(ItylosError::FileTooLarge.into());
        }
        let file_name = file_path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("secret_file.dat")
            .to_string();
        capsule.attachments.push(CapsuleFileV3 {
            name: file_name.clone(),
            mime: "application/octet-stream".to_string(),
            data: format!(
                "data:application/octet-stream;base64,{}",
                STANDARD.encode(file_bytes)
            ),
        });
        capsule.message = text;
        ui::print_file_loaded(&file_name, file_size);
    } else {
        if text.is_empty() {
            return Err(ItylosError::EmptyMessage.into());
        }
        capsule.message = text;
    }

    Ok(capsule)
}

fn parse_secret_url(url: &str) -> Result<(String, String)> {
    let (prefix, key_fragment) = url.split_once('#').ok_or(ItylosError::MissingUrlKey)?;
    let secret_id = prefix
        .rsplit('/')
        .next()
        .ok_or_else(|| ItylosError::Message("URL invalide".to_string()))?
        .to_string();
    let valid_secret_id = Regex::new(r"^[a-fA-F0-9]{32}$").expect("valid regex");
    if !valid_secret_id.is_match(&secret_id) {
        return Err(ItylosError::InvalidSecretId.into());
    }
    Ok((secret_id, key_fragment.to_string()))
}

#[cfg(test)]
fn validate_fetch_response(response: &FetchRes) -> Result<()> {
    validate_fetch_response_pre_password(response)?;
    if response.has_password {
        return Err(ItylosError::PasswordProtected.into());
    }
    Ok(())
}

fn validate_fetch_response_pre_password(response: &FetchRes) -> Result<()> {
    if !response.success {
        bail!(
            response
                .error
                .clone()
                .unwrap_or_else(|| "Erreur".to_string())
        );
    }
    if response.ttl == 0 {
        return Err(ItylosError::MissingTtl.into());
    }
    let payload = response
        .payload
        .as_deref()
        .ok_or_else(|| ItylosError::Message("payload absent".to_string()))?;
    validate_create_request_parts(
        payload,
        response.ttl,
        response.aad_hash.as_deref().unwrap_or_default(),
        response.has_password,
        response.pwd_salt.as_deref(),
    )?;
    Ok(())
}

fn validate_create_request_parts(
    payload: &str,
    ttl: u64,
    aad_hash: &str,
    has_password: bool,
    pwd_salt: Option<&str>,
) -> Result<()> {
    let payload_re = Regex::new(r"^[a-zA-Z0-9_-]+\.[a-zA-Z0-9_-]+$").expect("valid regex");
    let aad_hash_re = Regex::new(r"^[a-fA-F0-9]{64}$").expect("valid regex");

    if !payload_re.is_match(payload) {
        bail!("Payload invalide ou metadonnees manquantes.");
    }
    if normalize_ttl(ttl) != ttl {
        bail!("TTL invalide");
    }
    if !aad_hash_re.is_match(aad_hash) {
        bail!("aad_hash invalide");
    }
    let expected_aad_hash = crypto::compute_aad_hash_hex(ttl)?;
    if aad_hash.to_ascii_lowercase() != expected_aad_hash {
        bail!("aad_hash incoherent avec l'AAD attendu");
    }
    let pair_is_consistent = match (has_password, pwd_salt) {
        (true, Some(salt))
            if !salt.is_empty()
                && (URL_SAFE_NO_PAD.decode(salt).is_ok() || STANDARD.decode(salt).is_ok()) =>
        {
            true
        }
        (false, None) => true,
        (false, Some("")) => true,
        _ => false,
    };
    if !pair_is_consistent {
        bail!("has_password et pwd_salt doivent etre fournis ensemble");
    }
    if payload.len() > MAX_ATTACHMENT_BYTES {
        bail!("payload trop volumineux");
    }

    Ok(())
}

fn normalize_proof_document(json: Value) -> Result<Value> {
    if json.get("verification").is_some() {
        return Ok(json);
    }

    let envelope: BurnProofEnvelope =
        serde_json::from_value(json.get("proof").cloned().ok_or_else(|| {
            ItylosError::Message("Preuve malformee : bloc 'proof' absent ou invalide.".to_string())
        })?)
        .context("Preuve burn invalide")?;

    let mut payload = serde_json::to_value(envelope.payload).context("proof payload invalide")?;
    let verification = payload
        .get_mut("verification")
        .and_then(Value::as_object_mut)
        .ok_or_else(|| {
            ItylosError::Message(
                "Preuve malformee : bloc 'verification' absent ou invalide.".to_string(),
            )
        })?;
    verification.insert(
        "ed25519_signature".to_string(),
        Value::String(envelope.signature),
    );

    Ok(payload)
}

fn render_capsule(content: &str) -> Result<()> {
    let parsed = serde_json::from_str::<CapsuleV3>(content);
    if let Ok(mut capsule) = parsed
        && capsule.protocol == CAPSULE_PROTOCOL
    {
        if !capsule.message.is_empty() {
            println!("{}", capsule.message);
            capsule.message.zeroize();
        }

        for attachment in &mut capsule.attachments {
            let data = attachment
                .data
                .split_once(',')
                .map(|(_, encoded)| encoded)
                .unwrap_or(attachment.data.as_str());
            let mut decoded = STANDARD
                .decode(data)
                .context("Impossible de decoder le fichier joint")?;
            let safe_name = sanitize_filename(&attachment.name);
            std::fs::write(&safe_name, &decoded)
                .with_context(|| format!("Impossible de sauvegarder : {}", safe_name.display()))?;
            ui::print_file_extracted(&safe_name.display().to_string(), decoded.len());
            decoded.zeroize();
            attachment.data.zeroize();
            attachment.name.zeroize();
        }
    } else {
        println!("{}", content);
    }

    Ok(())
}

fn sanitize_filename(input: &str) -> PathBuf {
    let file_name = Path::new(input)
        .file_name()
        .and_then(|name| name.to_str())
        .filter(|name| !name.is_empty())
        .unwrap_or("secret_file.dat");
    PathBuf::from(file_name)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        env,
        sync::{Mutex, OnceLock},
    };
    use tempfile::tempdir;

    fn cwd_lock() -> &'static Mutex<()> {
        static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        LOCK.get_or_init(|| Mutex::new(()))
    }

    #[test]
    fn parse_secret_url_accepts_valid_link() {
        let (secret_id, key) =
            parse_secret_url("https://itylos.com/v/0123456789abcdef0123456789abcdef#secretkey")
                .expect("url should parse");
        assert_eq!(secret_id, "0123456789abcdef0123456789abcdef");
        assert_eq!(key, "secretkey");
    }

    #[test]
    fn parse_secret_url_rejects_missing_key_and_bad_id() {
        let missing_key = parse_secret_url("https://itylos.com/v/0123456789abcdef0123456789abcdef")
            .expect_err("missing key should fail");
        assert!(missing_key.to_string().contains("cle (#...) est manquante"));

        let bad_id =
            parse_secret_url("https://itylos.com/v/nothex#secret").expect_err("bad id should fail");
        assert!(bad_id.to_string().contains("malforme ou dangereux"));
    }

    #[test]
    fn validate_fetch_response_applies_guards() {
        let ok = FetchRes {
            success: true,
            payload: Some("abc_DEF.ghi-JKL".to_string()),
            aad_hash: Some(
                "09f7754b7ff9a179b9eccf1607134557d157c9f6722ab8817c29c20fc2343fcd".to_string(),
            ),
            has_password: false,
            pwd_salt: None,
            expires_at: Some("2026-03-28 15:00:00".to_string()),
            ttl: 3600,
            error: None,
        };
        validate_fetch_response(&ok).expect("valid response should pass");

        let no_ttl = FetchRes {
            ttl: 0,
            ..ok.clone()
        };
        assert!(
            validate_fetch_response(&no_ttl)
                .expect_err("ttl zero should fail")
                .to_string()
                .contains("TTL absent")
        );

        let password = FetchRes {
            has_password: true,
            pwd_salt: Some("c2FsdA==".to_string()),
            ..ok.clone()
        };
        assert!(
            validate_fetch_response(&password)
                .expect_err("password-protected should fail")
                .to_string()
                .contains("protegee par mot de passe")
        );

        let failed = FetchRes {
            success: false,
            error: Some("Erreur API".to_string()),
            ..ok
        };
        assert!(
            validate_fetch_response(&failed)
                .expect_err("failed response should fail")
                .to_string()
                .contains("Erreur API")
        );
    }

    #[test]
    fn build_capsule_requires_text_without_file() {
        let error = build_capsule(String::new(), None).expect_err("empty text should fail");
        assert!(error.to_string().contains("message est vide"));
    }

    #[test]
    fn build_capsule_embeds_attachment() {
        let dir = tempdir().expect("tempdir");
        let file_path = dir.path().join("note.txt");
        std::fs::write(&file_path, b"hello world").expect("write attachment");

        let capsule =
            build_capsule("bonjour".to_string(), Some(file_path)).expect("capsule should build");
        assert_eq!(capsule.protocol, CAPSULE_PROTOCOL);
        assert_eq!(capsule.message, "bonjour");
        assert_eq!(capsule.attachments.len(), 1);
        assert!(
            capsule.attachments[0]
                .data
                .starts_with("data:application/octet-stream;base64,")
        );
    }

    #[test]
    fn sanitize_filename_strips_path_components() {
        assert_eq!(
            sanitize_filename("../secret/../../note.txt"),
            PathBuf::from("note.txt")
        );
        assert_eq!(sanitize_filename(""), PathBuf::from("secret_file.dat"));
    }

    #[test]
    fn render_capsule_writes_attachment_in_current_directory() {
        let _guard = cwd_lock().lock().expect("cwd lock");
        let original_cwd = env::current_dir().expect("cwd");
        let dir = tempdir().expect("tempdir");
        env::set_current_dir(dir.path()).expect("set cwd");

        let content = serde_json::to_string(&CapsuleV3 {
            protocol: CAPSULE_PROTOCOL.to_string(),
            message: "bonjour".to_string(),
            attachments: vec![CapsuleFileV3 {
                name: "../proof.txt".to_string(),
                mime: "text/plain".to_string(),
                data: format!(
                    "data:text/plain;base64,{}",
                    STANDARD.encode("secret attachment")
                ),
            }],
        })
        .expect("serialize capsule");

        render_capsule(&content).expect("render should succeed");
        let extracted =
            std::fs::read_to_string(dir.path().join("proof.txt")).expect("attachment should exist");
        assert_eq!(extracted, "secret attachment");

        env::set_current_dir(original_cwd).expect("restore cwd");
    }

    #[test]
    fn render_capsule_falls_back_to_plain_text() {
        render_capsule("plain text message").expect("plain text should be accepted");
    }

    #[test]
    fn validate_create_request_parts_enforces_contract() {
        validate_create_request_parts(
            "abc_DEF.ghi-JKL",
            3600,
            "09f7754b7ff9a179b9eccf1607134557d157c9f6722ab8817c29c20fc2343fcd",
            false,
            None,
        )
        .expect("valid request should pass");

        assert!(
            validate_create_request_parts("bad/payload", 3600, "abcd", false, None)
                .expect_err("bad payload should fail")
                .to_string()
                .contains("Payload invalide")
        );
        assert!(
            validate_create_request_parts(
                "abc.def",
                1234,
                "09f7754b7ff9a179b9eccf1607134557d157c9f6722ab8817c29c20fc2343fcd",
                false,
                None,
            )
            .expect_err("bad ttl should fail")
            .to_string()
            .contains("TTL invalide")
        );
        assert!(
            validate_create_request_parts("abc.def", 3600, "zz", false, None)
                .expect_err("bad hash should fail")
                .to_string()
                .contains("aad_hash invalide")
        );
        assert!(
            validate_create_request_parts(
                "abc.def",
                3600,
                "ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff",
                false,
                None,
            )
            .expect_err("incoherent hash should fail")
            .to_string()
            .contains("aad_hash incoherent")
        );
        assert!(
            validate_create_request_parts(
                "abc.def",
                3600,
                "09f7754b7ff9a179b9eccf1607134557d157c9f6722ab8817c29c20fc2343fcd",
                true,
                None,
            )
            .expect_err("password pair should fail")
            .to_string()
            .contains("doivent etre fournis ensemble")
        );
        assert!(
            validate_create_request_parts(
                "abc.def",
                3600,
                "09f7754b7ff9a179b9eccf1607134557d157c9f6722ab8817c29c20fc2343fcd",
                true,
                Some("not-base64"),
            )
            .expect_err("bad salt should fail")
            .to_string()
            .contains("doivent etre fournis ensemble")
        );
    }

    #[test]
    fn normalize_proof_document_supports_burn_response_shape() {
        let burn_response = serde_json::json!({
            "success": true,
            "message": "Message purge du stockage actif.",
            "proof": {
                "proof_id": "49aa559ee6c15bd35b739761c2059b2e",
                "status": "DESTROYED",
                "signature": "sig-base64",
                "payload": {
                    "protocol_version": "itylos-proof-v2.0",
                    "proof_id": "49aa559ee6c15bd35b739761c2059b2e",
                    "secret_id": "0c720762cf11a975c31bef0c3d1da923",
                    "proof_type": "burned",
                    "status": "DESTROYED",
                    "resource_digest": "09f7754b7ff9a179b9eccf1607134557d157c9f6722ab8817c29c20fc2343fcd",
                    "lifecycle": {
                        "created_utc": "a",
                        "accessed_utc": "b",
                        "destroyed_utc": "c",
                        "expires_utc": "d"
                    },
                    "retention": {
                        "application_state": "PURGED_AND_OVERWRITTEN"
                    },
                    "verification": {
                        "ed25519_public_key_id": "itylos-proof-key-v2",
                        "ed25519_signature": "",
                        "pgp_anchor": null
                    }
                }
            }
        });

        let normalized = normalize_proof_document(burn_response).expect("proof should normalize");
        assert_eq!(
            normalized["verification"]["ed25519_signature"],
            Value::String("sig-base64".to_string())
        );
    }
}
