use anyhow::{Context, Result, bail};
use reqwest::blocking::Client;
use std::time::Duration;

use crate::types::{
    API_BURN, API_CREATE, API_FETCH, BurnReq, BurnRes, CreateReq, CreateRes, FetchRes,
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
        let body: CreateRes = response.json().context("reponse create invalide")?;
        if !status.is_success() {
            bail!(
                body.error
                    .unwrap_or_else(|| format!("Erreur API HTTP {status}"))
            );
        }
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
        let body: FetchRes = response.json().context("reponse fetch invalide")?;
        if !status.is_success() {
            bail!(
                body.error
                    .unwrap_or_else(|| format!("Erreur API HTTP {status}"))
            );
        }
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
        let body: BurnRes = response.json().context("reponse burn invalide")?;
        if !status.is_success() {
            bail!(
                body.error
                    .clone()
                    .unwrap_or_else(|| format!("Erreur API HTTP {status}"))
            );
        }
        Ok(body)
    }
}
