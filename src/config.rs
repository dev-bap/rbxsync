use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
pub struct Config {
    pub experience: Experience,

    #[serde(default, skip_serializing_if = "CodegenConfig::is_default")]
    pub codegen: CodegenConfig,

    #[serde(default, skip_serializing_if = "IconsConfig::is_default")]
    pub icons: IconsConfig,

    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub passes: BTreeMap<String, PassConfig>,

    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub badges: BTreeMap<String, BadgeConfig>,

    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub products: BTreeMap<String, ProductConfig>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Experience {
    pub universe_id: u64,
    pub creator: Creator,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Creator {
    #[serde(rename = "type")]
    pub creator_type: CreatorType,
    pub id: u64,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum CreatorType {
    User,
    Group,
}

impl std::fmt::Display for CreatorType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CreatorType::User => write!(f, "User"),
            CreatorType::Group => write!(f, "Group"),
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum CodegenStyle {
    #[default]
    Flat,
    Nested,
}

impl CodegenStyle {
    fn is_default(&self) -> bool {
        matches!(self, CodegenStyle::Flat)
    }
}

#[derive(Debug, Default, Deserialize, Serialize)]
pub struct CodegenPaths {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub passes: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub badges: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub products: Option<String>,
}

impl CodegenPaths {
    pub fn is_default(&self) -> bool {
        self.passes.is_none() && self.badges.is_none() && self.products.is_none()
    }
}

#[derive(Debug, Default, Deserialize, Serialize)]
pub struct CodegenConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output: Option<PathBuf>,

    /// Also generate a TypeScript definition file (.d.ts)
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub typescript: bool,

    /// Code generation style: "flat" (default) or "nested"
    #[serde(default, skip_serializing_if = "CodegenStyle::is_default")]
    pub style: CodegenStyle,

    #[serde(default, skip_serializing_if = "CodegenPaths::is_default")]
    pub paths: CodegenPaths,

    /// Extra entries injected into the generated file: `"path.to.key" = asset_id`
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub extra: BTreeMap<String, u64>,
}

impl CodegenConfig {
    fn is_default(&self) -> bool {
        self.output.is_none()
            && !self.typescript
            && self.style.is_default()
            && self.paths.is_default()
            && self.extra.is_empty()
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct IconsConfig {
    /// Apply alpha bleed to icons before uploading (default: true)
    #[serde(default = "default_true")]
    pub bleed: bool,

    /// Directory for downloaded icons (default: "icons")
    #[serde(default = "default_icon_dir")]
    pub dir: PathBuf,
}

impl Default for IconsConfig {
    fn default() -> Self {
        Self {
            bleed: true,
            dir: default_icon_dir(),
        }
    }
}

impl IconsConfig {
    fn is_default(&self) -> bool {
        self.bleed && self.dir == default_icon_dir()
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct PassConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub price: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub icon: Option<PathBuf>,
    #[serde(default = "default_true")]
    pub for_sale: bool,
    #[serde(default)]
    pub regional_pricing: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct BadgeConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub icon: Option<PathBuf>,
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
}

fn default_true() -> bool {
    true
}

fn default_icon_dir() -> PathBuf {
    PathBuf::from("icons")
}

/// Resolve the display name for a resource: use the explicit `name` field if set,
/// otherwise fall back to the TOML key.
pub fn resolve_name<'a>(config_name: Option<&'a str>, key: &'a str) -> &'a str {
    config_name.unwrap_or(key)
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ProductConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    pub price: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub icon: Option<PathBuf>,
    #[serde(default = "default_true")]
    pub for_sale: bool,
    #[serde(default)]
    pub regional_pricing: bool,
    #[serde(default)]
    pub store_page: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
}

impl Config {
    pub fn save(&self, path: &Path) -> Result<()> {
        let content = toml::to_string_pretty(self)?;
        std::fs::write(path, content)
            .with_context(|| format!("Failed to write {}", path.display()))?;
        Ok(())
    }

    pub fn load(path: &Path) -> Result<Self> {
        let content = std::fs::read_to_string(path)
            .with_context(|| format!("Failed to read {}", path.display()))?;
        let config: Config = toml::from_str(&content)
            .with_context(|| format!("Failed to parse {}", path.display()))?;

        let config_dir = path.parent().unwrap_or(Path::new("."));
        config.validate_icon_paths(config_dir)?;

        Ok(config)
    }

    fn validate_icon_paths(&self, config_dir: &Path) -> Result<()> {
        for (name, pass) in &self.passes {
            if let Some(icon) = &pass.icon {
                let full = config_dir.join(icon);
                if !full.exists() {
                    bail!(
                        "Pass '{}': icon path does not exist: {}",
                        name,
                        full.display()
                    );
                }
            }
        }
        for (name, badge) in &self.badges {
            if let Some(icon) = &badge.icon {
                let full = config_dir.join(icon);
                if !full.exists() {
                    bail!(
                        "Badge '{}': icon path does not exist: {}",
                        name,
                        full.display()
                    );
                }
            }
        }
        for (name, product) in &self.products {
            if let Some(icon) = &product.icon {
                let full = config_dir.join(icon);
                if !full.exists() {
                    bail!(
                        "Product '{}': icon path does not exist: {}",
                        name,
                        full.display()
                    );
                }
            }
        }
        Ok(())
    }

    pub fn default_template() -> String {
        r#"# rbxsync configuration

[experience]
universe_id = 0        # Your Roblox universe ID

[experience.creator]
type = "user"          # "user" or "group"
id = 0                 # Your Roblox user or group ID

# Codegen - generate a Luau module with asset IDs
# [codegen]
# output = "src/shared/GameIds.luau"
# typescript = false            # Also generate a .d.ts file
# style = "flat"               # "flat" (default) or "nested"
#                              # flat:   GameIds["passes.VIP"] — path-like keys
#                              # nested: GameIds.passes.VIP   — nested tables
#
# Custom paths — dot-separated, used as prefix (flat) or nesting (nested)
# [codegen.paths]
# passes = "player.vips"       # passes go under player.vips
# products = "shop.items"      # products go under shop.items
#
# Extra entries — pre-existing assets injected into the generated file
# [codegen.extra]
# "passes.legacy_vip" = 1234567   # dot path = asset id

# Icon settings
# [icons]
# bleed = true         # Apply alpha bleed (fixes resize artifacts)
# dir = "icons"        # Directory for downloaded icons

# Game Passes
# [passes.VIP]
# name = "VIP Pass"       # optional — defaults to "VIP"
# price = 499
# description = "VIP access"
# icon = "icons/vip.png"
# for_sale = true          # optional — defaults to true
# regional_pricing = false # optional — defaults to false
# path = "shop.specials"   # optional — override codegen path

# Badges
# [badges.Welcome]
# name = "Welcome Badge"  # optional — defaults to "Welcome"
# description = "Welcome to the game!"
# icon = "icons/welcome.png"
# enabled = true
# path = "rewards"          # optional — override codegen path

# Developer Products
# [products.Coins100]
# name = "100 Coins"      # optional — defaults to "Coins100"
# price = 99
# description = "100 coins"
# icon = "icons/coins.png"
# for_sale = true          # optional — defaults to true
# regional_pricing = false # optional — defaults to false
# store_page = false       # optional — defaults to false
# path = "shop.specials"   # optional — override codegen path
"#
        .to_string()
    }
}
