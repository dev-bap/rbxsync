pub mod badges;
pub mod models;
pub mod passes;
pub mod products;

use std::time::Duration;

use anyhow::{bail, Result};
use reqwest::{multipart, Client, Response, StatusCode};

use models::AssetDeliveryResponse;

/// Which pricing automation a write enforces.
///
/// Roblox publishes two form fields for this, `isRegionalPricingEnabled` and
/// `isManagedPricingEnabled`, and documents them as mutually exclusive: the
/// regional one is marked deprecated on every write endpoint, and the
/// developer-product one says it "should not be used when setting
/// isManagedPricingEnabled". Managed pricing is the successor, and it bundles
/// regional pricing with price optimization under one opt-in.
///
/// An enum rather than two booleans so a request that sets both cannot be
/// built in the first place.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Pricing {
    /// Send neither field, leaving whatever Roblox already has. That matters
    /// because Roblox now enables managed pricing by itself on passes: a
    /// write that always sent one of these fields would silently turn off
    /// something nobody asked to turn off.
    Untouched,
    /// Send the deprecated `isRegionalPricingEnabled`.
    Regional(bool),
    /// Send `isManagedPricingEnabled`.
    Managed(bool),
}

impl Pricing {
    /// How a config's two pricing keys resolve to the one field sent.
    ///
    /// `managed_pricing` wins when set, because the two cannot both be sent
    /// and it is the field Roblox still maintains. A config setting both to a
    /// conflicting pair is refused earlier, so this never has to guess which
    /// one the author meant.
    pub fn from_config(regional: bool, managed: Option<bool>) -> Self {
        match (managed, regional) {
            (Some(v), _) => Self::Managed(v),
            // The default. An implicit `regional_pricing = false` states no
            // intent, so it sends nothing rather than writing the deprecated
            // field as false on every sync.
            (None, false) => Self::Untouched,
            (None, true) => Self::Regional(true),
        }
    }

    /// Add that field to a form, or leave the form alone.
    pub fn apply(self, form: multipart::Form) -> multipart::Form {
        match self {
            Self::Untouched => form,
            Self::Regional(v) => form.text("isRegionalPricingEnabled", v.to_string()),
            Self::Managed(v) => form.text("isManagedPricingEnabled", v.to_string()),
        }
    }
}

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
        self.api_key.as_deref().ok_or_else(|| {
            anyhow::anyhow!("--api-key or RBXSYNC_API_KEY env var is required for this operation")
        })
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
