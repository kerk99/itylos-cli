use anyhow::{Context, Result, bail};
use reqwest::blocking::Client;
use std::time::Duration;

use crate::types::{
    API_BURN, API_CREATE, API_FETCH, API_PROOF, ApiErrorRes, BurnReq, BurnRes, CreateReq,
    CreateRes, FetchRes,
};

pub struct ItylosApi {
    client: Client,
}

impl ItylosApi {
    pub fn new() -> Result<Self> {
        let client = Client::builder()
            .timeout(Duration::from_secs(20))
            .build()
            .context("impossible d'initialiser le client HTTP")?;
        Ok(Self { client })
    }

    pub fn create_secret(&self, request: &CreateReq) -> Result<CreateRes> {
        let response = self
            .client
            .post(API_CREATE)
            .json(request)
            .send()
            .context("Erreur reseau")?;
        let status = response.status();
        if !status.is_success() {
            let err: ApiErrorRes = response.json().unwrap_or(ApiErrorRes {
                success: false,
                error: Some(format!("Erreur API HTTP {status}")),
            });
            bail!(
                err.error
                    .unwrap_or_else(|| format!("Erreur API HTTP {status}"))
            );
        }
        let body: CreateRes = response.json().context("reponse create invalide")?;
        Ok(body)
    }

    pub fn fetch_secret(&self, secret_id: &str) -> Result<FetchRes> {
        let response = self
            .client
            .get(API_FETCH)
            .query(&[("id", secret_id)])
            .send()
            .context("Erreur reseau")?;
        let status = response.status();
        if !status.is_success() {
            let err: ApiErrorRes = response.json().unwrap_or(ApiErrorRes {
                success: false,
                error: Some(format!("Erreur API HTTP {status}")),
            });
            bail!(
                err.error
                    .unwrap_or_else(|| format!("Erreur API HTTP {status}"))
            );
        }
        let body: FetchRes = response.json().context("reponse fetch invalide")?;
        Ok(body)
    }

    pub fn burn_secret_with_proof(&self, secret_id: &str) -> Result<BurnRes> {
        let request = BurnReq {
            id: secret_id.to_string(),
        };
        let response = self
            .client
            .post(API_BURN)
            .json(&request)
            .send()
            .context("Erreur reseau")?;
        let status = response.status();
        if !status.is_success() {
            let err: ApiErrorRes = response.json().unwrap_or(ApiErrorRes {
                success: false,
                error: Some(format!("Erreur API HTTP {status}")),
            });
            bail!(
                err.error
                    .unwrap_or_else(|| format!("Erreur API HTTP {status}"))
            );
        }
        let body: BurnRes = response.json().context("reponse burn invalide")?;
        Ok(body)
    }

    pub fn fetch_proof(&self, proof_id: &str) -> Result<serde_json::Value> {
        let response = self
            .client
            .get(API_PROOF)
            .query(&[("id", proof_id)])
            .send()
            .context("Erreur reseau")?;
        let status = response.status();
        if !status.is_success() {
            let err: ApiErrorRes = response.json().unwrap_or(ApiErrorRes {
                success: false,
                error: Some(format!("Erreur API HTTP {status}")),
            });
            bail!(
                err.error
                    .unwrap_or_else(|| format!("Erreur API HTTP {status}"))
            );
        }
        let body: serde_json::Value = response.json().context("reponse proof invalide")?;
        Ok(body)
    }
}
