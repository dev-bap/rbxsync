use std::path::Path;

use anyhow::{bail, Result};
use reqwest::multipart;

use super::models::{DeveloperProduct, ListDeveloperProductsResponse};
use super::RbxClient;

impl RbxClient {
    pub async fn list_all_developer_products(&self) -> Result<Vec<DeveloperProduct>> {
        let api_key = self.api_key_header()?.to_string();
        let mut all_products = Vec::new();
        let mut page_token: Option<String> = None;

        loop {
            let mut url = format!(
                "https://apis.roblox.com/developer-products/v2/universes/{}/developer-products/creator?pageSize=50",
                self.universe_id
            );
            if let Some(token) = &page_token {
                url.push_str(&format!("&pageToken={}", token));
            }

            let list: ListDeveloperProductsResponse = self
                .execute_json(|| async {
                    Ok(self
                        .client
                        .get(&url)
                        .header("x-api-key", &api_key)
                        .send()
                        .await?)
                })
                .await?;

            all_products.extend(list.developer_products);

            match list.next_page_token {
                Some(token) if !token.is_empty() => page_token = Some(token),
                _ => break,
            }
        }

        Ok(all_products)
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn create_developer_product(
        &self,
        name: &str,
        description: Option<&str>,
        price: u64,
        icon_path: Option<&Path>,
        is_for_sale: bool,
        is_regional_pricing_enabled: bool,
    ) -> Result<DeveloperProduct> {
        let api_key = self.api_key_header()?.to_string();
        let url = format!(
            "https://apis.roblox.com/developer-products/v2/universes/{}/developer-products",
            self.universe_id
        );

        let mut form = multipart::Form::new()
            .text("name", name.to_string())
            .text("description", description.unwrap_or("").to_string())
            .text("isForSale", is_for_sale.to_string())
            .text(
                "isRegionalPricingEnabled",
                is_regional_pricing_enabled.to_string(),
            )
            .text("price", price.to_string());

        if let Some(path) = icon_path {
            let bytes = crate::icon::process_icon(path, self.bleed)?;
            let part = multipart::Part::bytes(bytes)
                .file_name("icon.png")
                .mime_str("image/png")?;
            form = form.part("imageFile", part);
        }

        let response = self
            .client
            .post(&url)
            .header("x-api-key", &api_key)
            .multipart(form)
            .send()
            .await?;

        let status = response.status();
        let body = response.text().await?;
        if !status.is_success() {
            bail!("API error {}: {}", status, body);
        }

        Ok(serde_json::from_str(&body)?)
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn update_developer_product(
        &self,
        id: u64,
        name: &str,
        description: Option<&str>,
        price: u64,
        icon_path: Option<&Path>,
        is_for_sale: bool,
        is_regional_pricing_enabled: bool,
        store_page_enabled: bool,
    ) -> Result<DeveloperProduct> {
        let api_key = self.api_key_header()?.to_string();
        let url = format!(
            "https://apis.roblox.com/developer-products/v2/universes/{}/developer-products/{}",
            self.universe_id, id
        );

        // The API validates isForSale against the CURRENT remote state, not the
        // state being sent. So setting isForSale=false while the product is
        // currently on the store page fails with InvalidIsForSale, even if
        // storePageEnabled=false is sent in the same request.
        // Workaround: first remove from store page, then set off sale.
        if !is_for_sale {
            let disable_store_form = multipart::Form::new()
                .text("name", name.to_string())
                .text("description", description.unwrap_or("").to_string())
                .text("isForSale", "true")
                .text(
                    "isRegionalPricingEnabled",
                    is_regional_pricing_enabled.to_string(),
                )
                .text("storePageEnabled", "false")
                .text("price", price.to_string());

            let resp = self
                .client
                .patch(&url)
                .header("x-api-key", &api_key)
                .multipart(disable_store_form)
                .send()
                .await?;

            if !resp.status().is_success() && !resp.status().is_redirection() {
                let body = resp.text().await.unwrap_or_default();
                bail!("API error while disabling store page: {}", body);
            }
        }

        let effective_store_page = store_page_enabled && is_for_sale;

        let mut form = multipart::Form::new()
            .text("name", name.to_string())
            .text("description", description.unwrap_or("").to_string())
            .text("isForSale", is_for_sale.to_string())
            .text(
                "isRegionalPricingEnabled",
                is_regional_pricing_enabled.to_string(),
            )
            .text("storePageEnabled", effective_store_page.to_string())
            .text("price", price.to_string());

        if let Some(path) = icon_path {
            let bytes = crate::icon::process_icon(path, self.bleed)?;
            let part = multipart::Part::bytes(bytes)
                .file_name("icon.png")
                .mime_str("image/png")?;
            form = form.part("imageFile", part);
        }

        let response = self
            .client
            .patch(&url)
            .header("x-api-key", &api_key)
            .multipart(form)
            .send()
            .await?;

        let status = response.status();
        let body = response.text().await?;
        if !status.is_success() {
            bail!("API error {}: {}", status, body);
        }

        if body.is_empty() {
            return self.get_developer_product(id).await;
        }

        Ok(serde_json::from_str(&body)?)
    }

    pub async fn get_developer_product(&self, id: u64) -> Result<DeveloperProduct> {
        let api_key = self.api_key_header()?.to_string();
        let url = format!(
            "https://apis.roblox.com/developer-products/v2/universes/{}/developer-products/{}/creator",
            self.universe_id, id
        );

        self.execute_json(|| async {
            Ok(self
                .client
                .get(&url)
                .header("x-api-key", &api_key)
                .send()
                .await?)
        })
        .await
    }
}
