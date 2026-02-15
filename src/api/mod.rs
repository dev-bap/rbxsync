pub mod badges;
pub mod models;
pub mod passes;
pub mod products;

use std::time::Duration;

use anyhow::{bail, Result};
use reqwest::{Client, Response, StatusCode};

use models::AssetDeliveryResponse;

pub struct RbxClient {
    pub client: Client,
    pub api_key: Option<String>,
    pub universe_id: u64,
    pub bleed: bool,
}

impl RbxClient {
    pub fn new(api_key: Option<String>, universe_id: u64, bleed: bool) -> Self {
        Self {
            client: Client::builder().gzip(true).build().unwrap(),
            api_key,
            universe_id,
            bleed,
        }
    }

    /// API key header for Open Cloud endpoints.
    pub fn api_key_header(&self) -> Result<&str> {
        self.api_key
            .as_deref()
            .ok_or_else(|| anyhow::anyhow!("--api-key is required for this operation"))
    }

    pub async fn execute_with_retry<F, Fut>(&self, mut make_request: F) -> Result<Response>
    where
        F: FnMut() -> Fut,
        Fut: std::future::Future<Output = Result<Response>>,
    {
        let max_retries = 3;
        let mut attempt = 0;

        loop {
            let response = make_request().await?;
            let status = response.status();

            if status.is_success() || status == StatusCode::NO_CONTENT {
                return Ok(response);
            }

            let should_retry = status == StatusCode::TOO_MANY_REQUESTS || status.is_server_error();

            if !should_retry || attempt >= max_retries {
                let body = response.text().await.unwrap_or_default();
                bail!("API error {}: {}", status, body);
            }

            let retry_after = response
                .headers()
                .get("retry-after")
                .and_then(|v| v.to_str().ok())
                .and_then(|v| v.parse::<u64>().ok());

            let delay = retry_after.unwrap_or(1 << attempt);
            tokio::time::sleep(Duration::from_secs(delay)).await;
            attempt += 1;
        }
    }

    pub async fn execute_json<T: serde::de::DeserializeOwned, F, Fut>(
        &self,
        make_request: F,
    ) -> Result<T>
    where
        F: FnMut() -> Fut,
        Fut: std::future::Future<Output = Result<Response>>,
    {
        let response = self.execute_with_retry(make_request).await?;
        let body = response.text().await?;
        let parsed: T = serde_json::from_str(&body)
            .map_err(|e| anyhow::anyhow!("Failed to parse response: {}\nBody: {}", e, body))?;
        Ok(parsed)
    }

    /// Download an asset's raw bytes from Roblox via the asset delivery API.
    pub async fn download_asset(&self, asset_id: u64) -> Result<Vec<u8>> {
        let api_key = self.api_key_header()?.to_string();
        let url = format!(
            "https://apis.roblox.com/asset-delivery-api/v1/assetId/{}",
            asset_id
        );
        let resp: AssetDeliveryResponse = self
            .execute_json(|| async {
                Ok(self
                    .client
                    .get(&url)
                    .header("x-api-key", &api_key)
                    .send()
                    .await?)
            })
            .await?;

        let bytes = self
            .client
            .get(&resp.location)
            .send()
            .await?
            .bytes()
            .await?;
        Ok(bytes.to_vec())
    }
}
