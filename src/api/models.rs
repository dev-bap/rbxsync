use serde::{Deserialize, Serialize};

// ── Shared ──

#[derive(Debug, Deserialize, Serialize)]
pub struct PriceInformation {
    #[serde(rename = "defaultPriceInRobux")]
    pub default_price_in_robux: Option<u64>,
}

// ── Game Passes ──

#[derive(Debug, Deserialize, Serialize)]
pub struct GamePass {
    #[serde(rename = "gamePassId")]
    pub id: Option<u64>,
    pub name: Option<String>,
    pub description: Option<String>,
    #[serde(rename = "isForSale")]
    pub is_for_sale: Option<bool>,
    #[serde(rename = "iconAssetId")]
    pub icon_asset_id: Option<u64>,
    #[serde(rename = "priceInformation")]
    pub price_information: Option<PriceInformation>,
}

impl GamePass {
    pub fn price(&self) -> Option<u64> {
        self.price_information.as_ref()?.default_price_in_robux
    }
}

#[derive(Debug, Deserialize)]
pub struct ListGamePassesResponse {
    #[serde(rename = "gamePasses", default)]
    pub game_passes: Vec<GamePass>,
    #[serde(rename = "nextPageToken")]
    pub next_page_token: Option<String>,
}

// ── Badges ──

#[derive(Debug, Deserialize, Serialize)]
pub struct Badge {
    pub id: Option<u64>,
    pub name: Option<String>,
    pub description: Option<String>,
    pub enabled: Option<bool>,
    #[serde(rename = "iconImageId")]
    pub icon_image_id: Option<u64>,
}

#[derive(Debug, Deserialize)]
pub struct ListBadgesResponse {
    pub data: Option<Vec<Badge>>,
    #[serde(rename = "nextPageCursor")]
    pub next_page_cursor: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct BadgeIconResponse {
    #[serde(rename = "targetId")]
    pub target_id: Option<u64>,
}

// ── Developer Products ──

#[derive(Debug, Deserialize, Serialize)]
pub struct DeveloperProduct {
    #[serde(rename = "productId")]
    pub id: Option<u64>,
    pub name: Option<String>,
    pub description: Option<String>,
    #[serde(rename = "iconImageAssetId")]
    pub icon_image_asset_id: Option<u64>,
    #[serde(rename = "isForSale")]
    pub is_for_sale: Option<bool>,
    #[serde(rename = "storePageEnabled")]
    pub store_page_enabled: Option<bool>,
    #[serde(rename = "priceInformation")]
    pub price_information: Option<PriceInformation>,
}

impl DeveloperProduct {
    pub fn price(&self) -> Option<u64> {
        self.price_information.as_ref()?.default_price_in_robux
    }
}

#[derive(Debug, Deserialize)]
pub struct ListDeveloperProductsResponse {
    #[serde(rename = "developerProducts", default)]
    pub developer_products: Vec<DeveloperProduct>,
    #[serde(rename = "nextPageToken")]
    pub next_page_token: Option<String>,
}

// ── Asset Delivery ──

#[derive(Debug, Deserialize)]
pub struct AssetDeliveryResponse {
    pub location: String,
}
