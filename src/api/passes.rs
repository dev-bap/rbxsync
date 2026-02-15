use std::path::Path;

use anyhow::{bail, Result};
use reqwest::multipart;

use super::models::{GamePass, ListGamePassesResponse};
use super::RbxClient;

impl RbxClient {
    pub async fn list_all_game_passes(&self) -> Result<Vec<GamePass>> {
        let api_key = self.api_key_header()?.to_string();
        let mut all_passes = Vec::new();
        let mut page_token: Option<String> = None;

        loop {
            let mut url = format!(
                "https://apis.roblox.com/game-passes/v1/universes/{}/game-passes/creator?pageSize=100",
                self.universe_id
            );
            if let Some(token) = &page_token {
                url.push_str(&format!("&pageToken={}", token));
            }

            let list: ListGamePassesResponse = self
                .execute_json(|| async {
                    Ok(self
                        .client
                        .get(&url)
                        .header("x-api-key", &api_key)
                        .send()
                        .await?)
                })
                .await?;

            all_passes.extend(list.game_passes);

            match list.next_page_token {
                Some(token) if !token.is_empty() => page_token = Some(token),
                _ => break,
            }
        }

        Ok(all_passes)
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn create_game_pass(
        &self,
        name: &str,
        description: Option<&str>,
        price: Option<u64>,
        icon_path: Option<&Path>,
        is_for_sale: bool,
        is_regional_pricing_enabled: bool,
    ) -> Result<GamePass> {
        let api_key = self.api_key_header()?.to_string();
        let url = format!(
            "https://apis.roblox.com/game-passes/v1/universes/{}/game-passes",
            self.universe_id
        );

        let mut form = multipart::Form::new()
            .text("name", name.to_string())
            .text("description", description.unwrap_or("").to_string())
            .text("isForSale", is_for_sale.to_string())
            .text(
                "isRegionalPricingEnabled",
                is_regional_pricing_enabled.to_string(),
            );

        if let Some(p) = price {
            form = form.text("price", p.to_string());
        }
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
    pub async fn update_game_pass(
        &self,
        id: u64,
        name: &str,
        description: Option<&str>,
        price: Option<u64>,
        icon_path: Option<&Path>,
        is_for_sale: bool,
        is_regional_pricing_enabled: bool,
    ) -> Result<GamePass> {
        let api_key = self.api_key_header()?.to_string();
        let url = format!(
            "https://apis.roblox.com/game-passes/v1/universes/{}/game-passes/{}",
            self.universe_id, id
        );

        let mut form = multipart::Form::new()
            .text("name", name.to_string())
            .text("description", description.unwrap_or("").to_string())
            .text("isForSale", is_for_sale.to_string())
            .text(
                "isRegionalPricingEnabled",
                is_regional_pricing_enabled.to_string(),
            );

        if let Some(p) = price {
            form = form.text("price", p.to_string());
        }
        if let Some(path) = icon_path {
            let bytes = crate::icon::process_icon(path, self.bleed)?;
            let part = multipart::Part::bytes(bytes)
                .file_name("icon.png")
                .mime_str("image/png")?;
            form = form.part("file", part);
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

        // Update returns 204 No Content, so body may be empty
        if body.is_empty() {
            // Fetch the updated pass to return it
            return self.get_game_pass(id).await;
        }

        Ok(serde_json::from_str(&body)?)
    }

    pub async fn get_game_pass(&self, id: u64) -> Result<GamePass> {
        let api_key = self.api_key_header()?.to_string();
        let url = format!(
            "https://apis.roblox.com/game-passes/v1/universes/{}/game-passes/{}/creator",
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
