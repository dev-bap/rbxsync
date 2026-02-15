use std::path::Path;

use anyhow::{bail, Result};
use reqwest::multipart;

use super::models::{Badge, BadgeIconResponse, ListBadgesResponse};
use super::RbxClient;

impl RbxClient {
    pub async fn list_all_badges(&self, universe_id: u64) -> Result<Vec<Badge>> {
        let api_key = self.api_key_header()?.to_string();
        let mut all_badges = Vec::new();
        let mut cursor: Option<String> = None;

        loop {
            let mut url = format!(
                "https://badges.roblox.com/v1/universes/{}/badges?limit=100&sortOrder=Asc",
                universe_id
            );
            if let Some(c) = &cursor {
                url.push_str(&format!("&cursor={}", c));
            }

            let list: ListBadgesResponse = self
                .execute_json(|| async {
                    Ok(self
                        .client
                        .get(&url)
                        .header("x-api-key", &api_key)
                        .send()
                        .await?)
                })
                .await?;

            if let Some(data) = list.data {
                all_badges.extend(data);
            }

            match list.next_page_cursor {
                Some(c) if !c.is_empty() => cursor = Some(c),
                _ => break,
            }
        }

        Ok(all_badges)
    }

    pub async fn get_badge(&self, badge_id: u64) -> Result<Badge> {
        let api_key = self.api_key_header()?.to_string();
        let url = format!("https://badges.roblox.com/v1/badges/{}", badge_id);

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

    pub async fn create_badge(
        &self,
        name: &str,
        description: Option<&str>,
        icon_path: Option<&Path>,
        payment_source: u32,
        expected_cost: u64,
    ) -> Result<Badge> {
        let api_key = self.api_key_header()?.to_string();
        let url = format!(
            "https://apis.roblox.com/legacy-badges/v1/universes/{}/badges",
            self.universe_id
        );

        let mut form = multipart::Form::new()
            .text("name", name.to_string())
            .text("description", description.unwrap_or("").to_string())
            .text("paymentSourceType", payment_source.to_string())
            .text("expectedCost", expected_cost.to_string())
            .text("isActive", "true".to_string());

        if let Some(path) = icon_path {
            let bytes = crate::icon::process_icon(path, self.bleed)?;
            let part = multipart::Part::bytes(bytes)
                .file_name("icon.png")
                .mime_str("image/png")?;
            form = form.part("files", part);
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

    pub async fn update_badge(
        &self,
        badge_id: u64,
        name: &str,
        description: Option<&str>,
        enabled: bool,
    ) -> Result<Badge> {
        let api_key = self.api_key_header()?.to_string();
        let url = format!(
            "https://apis.roblox.com/legacy-badges/v1/badges/{}",
            badge_id
        );

        let body = serde_json::json!({
            "name": name,
            "description": description.unwrap_or(""),
            "enabled": enabled,
        });

        let response = self
            .client
            .patch(&url)
            .header("x-api-key", &api_key)
            .json(&body)
            .send()
            .await?;

        let status = response.status();
        let resp_body = response.text().await?;
        if !status.is_success() {
            bail!("API error {}: {}", status, resp_body);
        }

        Ok(serde_json::from_str(&resp_body)?)
    }

    pub async fn update_badge_icon(
        &self,
        badge_id: u64,
        icon_path: &Path,
    ) -> Result<BadgeIconResponse> {
        let api_key = self.api_key_header()?.to_string();
        let url = format!(
            "https://apis.roblox.com/legacy-publish/v1/badges/{}/icon",
            badge_id
        );

        let bytes = crate::icon::process_icon(icon_path, self.bleed)?;
        let part = multipart::Part::bytes(bytes)
            .file_name("icon.png")
            .mime_str("image/png")?;
        let form = multipart::Form::new().part("Files", part);

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
}
