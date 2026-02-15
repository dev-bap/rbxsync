use std::collections::BTreeMap;
use std::path::Path;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

#[derive(Debug, Default, PartialEq, Deserialize, Serialize)]
pub struct Lockfile {
    pub version: u32,
    pub universe_id: u64,

    #[serde(default)]
    pub passes: BTreeMap<String, PassLock>,

    #[serde(default)]
    pub badges: BTreeMap<String, BadgeLock>,

    #[serde(default)]
    pub products: BTreeMap<String, ProductLock>,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct PassLock {
    pub id: u64,
    pub name: String,
    pub price: Option<u64>,
    pub description: Option<String>,
    pub icon_asset_id: Option<u64>,
    pub icon_hash: Option<String>,
    #[serde(default = "default_true")]
    pub for_sale: bool,
    #[serde(default)]
    pub regional_pricing: bool,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct BadgeLock {
    pub id: u64,
    pub name: String,
    pub description: Option<String>,
    pub enabled: bool,
    pub icon_asset_id: Option<u64>,
    pub icon_hash: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct ProductLock {
    pub id: u64,
    pub name: String,
    pub price: u64,
    pub description: Option<String>,
    pub icon_asset_id: Option<u64>,
    pub icon_hash: Option<String>,
    #[serde(default = "default_true")]
    pub for_sale: bool,
    #[serde(default)]
    pub regional_pricing: bool,
    #[serde(default)]
    pub store_page: bool,
}

fn default_true() -> bool {
    true
}

pub const LOCKFILE_NAME: &str = "rbxsync.lock.toml";

impl Lockfile {
    pub fn load(path: &Path) -> Result<Self> {
        if !path.exists() {
            return Ok(Self::default());
        }
        let content = std::fs::read_to_string(path)
            .with_context(|| format!("Failed to read {}", path.display()))?;
        let lockfile: Lockfile = toml::from_str(&content)
            .with_context(|| format!("Failed to parse {}", path.display()))?;
        Ok(lockfile)
    }

    pub fn save(&self, path: &Path) -> Result<()> {
        let content = toml::to_string_pretty(self)?;
        std::fs::write(path, content)
            .with_context(|| format!("Failed to write {}", path.display()))?;
        Ok(())
    }
}
