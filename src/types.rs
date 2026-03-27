use serde::{Deserialize, Serialize};
use std::path::PathBuf;

pub const VERSION: &str = "v2.0.0-rust";
pub const DOMAIN: &str = "https://almowatin.org";
pub const API_CREATE: &str = "https://almowatin.org/api/v2/create_secret";
pub const API_FETCH: &str = "https://almowatin.org/api/v2/fetch_secret";
pub const API_BURN: &str = "https://almowatin.org/api/v2/burn_secret";
pub const SERVER_PUB_KEY_B64: &str = "tsIkULXxSVudU1ZkJ3u5IpXN+11WpaVeog/4tG8qacI=";
pub const MAX_ATTACHMENT_BYTES: usize = 8 * 1024 * 1024;
pub const CAPSULE_PROTOCOL: &str = "ITYLOS_CAPSULE_V3_MULTI";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapsuleV3 {
    pub protocol: String,
    pub message: String,
    pub attachments: Vec<CapsuleFileV3>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapsuleFileV3 {
    pub name: String,
    pub mime: String,
    pub data: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaddedPayload {
    pub content: String,
    pub noise: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AadV2 {
    pub v: String,
    pub alg: String,
    pub ttl: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateReq {
    pub payload: String,
    pub ttl: u64,
    pub aad_hash: String,
    #[serde(skip_serializing_if = "is_false")]
    pub has_password: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pwd_salt: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateRes {
    pub success: bool,
    pub secret_id: String,
    pub proof_id: Option<String>,
    pub proof_token: Option<String>,
    pub expires_in: Option<u64>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FetchRes {
    pub success: bool,
    pub payload: Option<String>,
    pub aad_hash: Option<String>,
    pub has_password: bool,
    pub pwd_salt: Option<String>,
    pub expires_at: Option<String>,
    pub ttl: u64,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BurnReq {
    pub id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BurnRes {
    pub success: bool,
    pub message: Option<String>,
    pub proof: Option<BurnProofEnvelope>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BurnProofEnvelope {
    pub proof_id: String,
    pub status: String,
    pub signature: String,
    pub payload: BurnProofPayload,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BurnProofPayload {
    pub protocol_version: String,
    pub proof_id: String,
    pub secret_id: String,
    pub proof_type: String,
    pub status: String,
    pub resource_digest: String,
    pub lifecycle: BurnProofLifecycle,
    pub retention: BurnProofRetention,
    pub verification: BurnProofVerification,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BurnProofLifecycle {
    pub created_utc: String,
    pub accessed_utc: String,
    pub destroyed_utc: String,
    pub expires_utc: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BurnProofRetention {
    pub application_state: String,
    pub notes: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BurnProofVerification {
    pub ed25519_public_key_id: String,
    pub ed25519_signature: String,
    pub pgp_anchor: Option<String>,
}

#[derive(Debug, Clone)]
pub struct SendOptions {
    pub text: String,
    pub file: Option<PathBuf>,
    pub ttl: Ttl,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Ttl {
    OneHour,
    TwentyFourHours,
    SevenDays,
}

impl Ttl {
    pub fn parse(raw: &str) -> Self {
        match raw {
            "24h" => Self::TwentyFourHours,
            "7j" => Self::SevenDays,
            _ => Self::OneHour,
        }
    }

    pub fn seconds(self) -> u64 {
        match self {
            Self::OneHour => 3_600,
            Self::TwentyFourHours => 86_400,
            Self::SevenDays => 604_800,
        }
    }
}

fn is_false(value: &bool) -> bool {
    !*value
}

pub fn normalize_ttl(ttl: u64) -> u64 {
    match ttl {
        3_600 | 86_400 | 604_800 => ttl,
        _ => 3_600,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ttl_parsing_matches_supported_values() {
        assert_eq!(Ttl::parse("1h"), Ttl::OneHour);
        assert_eq!(Ttl::parse("24h"), Ttl::TwentyFourHours);
        assert_eq!(Ttl::parse("7j"), Ttl::SevenDays);
        assert_eq!(Ttl::parse("unexpected"), Ttl::OneHour);
    }

    #[test]
    fn ttl_seconds_match_go_contract() {
        assert_eq!(Ttl::OneHour.seconds(), 3_600);
        assert_eq!(Ttl::TwentyFourHours.seconds(), 86_400);
        assert_eq!(Ttl::SevenDays.seconds(), 604_800);
    }

    #[test]
    fn create_request_omits_false_password_fields() {
        let req = CreateReq {
            payload: "payload".to_string(),
            ttl: 3600,
            aad_hash: "deadbeef".to_string(),
            has_password: false,
            pwd_salt: None,
        };

        let json = serde_json::to_value(req).expect("json should serialize");
        assert!(json.get("has_password").is_none());
        assert!(json.get("pwd_salt").is_none());
    }
}
