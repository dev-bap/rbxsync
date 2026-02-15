use std::path::{Path, PathBuf};

use anyhow::{bail, Context, Result};
use colored::Colorize;

use crate::api::RbxClient;
use crate::cli::Cli;
use crate::config::{
    BadgeConfig, CodegenConfig, Config, Creator, CreatorType, Experience, IconsConfig, PassConfig,
    ProductConfig,
};
use crate::lockfile::{BadgeLock, Lockfile, PassLock, ProductLock, LOCKFILE_NAME};

pub async fn run(cli: &Cli, from_remote: bool, universe_id: Option<u64>) -> Result<()> {
    let config_path = &cli.config;

    if !from_remote {
        if config_path.exists() {
            bail!(
                "{} already exists. Remove it first or use a different path with --config.",
                config_path.display()
            );
        }

        let template = Config::default_template();
        std::fs::write(config_path, template)
            .with_context(|| format!("Failed to write {}", config_path.display()))?;

        println!("{} Created {}", "✓".green(), config_path.display());
        println!(
            "Edit the file to configure your universe and resources, then run `rbxsync sync`."
        );
        return Ok(());
    }

    // --from-remote mode
    let universe_id = universe_id
        .ok_or_else(|| anyhow::anyhow!("--universe-id is required with --from-remote"))?;

    let client = RbxClient::new(cli.api_key.clone(), universe_id, true);
    let config_dir = config_path.parent().unwrap_or(Path::new("."));
    let icons_config = IconsConfig::default();

    println!("Fetching remote resources...");

    let remote_passes = client.list_all_game_passes().await?;
    let remote_badges = client.list_all_badges(universe_id).await?;
    let remote_products = client.list_all_developer_products().await?;

    let mut passes = std::collections::BTreeMap::new();
    let mut pass_locks = std::collections::BTreeMap::new();
    for pass in &remote_passes {
        let name = pass.name.as_deref().unwrap_or("unnamed");
        let id = pass.id.unwrap_or(0);

        if passes.contains_key(name) {
            println!(
                "{} Duplicate pass name '{}' (id: {}) — skipping (only the first is kept)",
                "!".yellow(),
                name,
                id
            );
            continue;
        }

        let icon_asset_id = pass.icon_asset_id;
        let (icon_path, icon_hash) = download_icon(
            &client,
            &icons_config,
            config_dir,
            "pass",
            id,
            name,
            &icon_asset_id,
        )
        .await?;

        let is_for_sale = pass.is_for_sale.unwrap_or(true);
        passes.insert(
            name.to_string(),
            PassConfig {
                name: None,
                price: pass.price(),
                description: pass.description.clone(),
                icon: icon_path,
                for_sale: is_for_sale,
                regional_pricing: false,
                path: None,
            },
        );
        pass_locks.insert(
            name.to_string(),
            PassLock {
                id,
                name: name.to_string(),
                price: pass.price(),
                description: pass.description.clone(),
                icon_asset_id,
                icon_hash,
                for_sale: is_for_sale,
                regional_pricing: false,
            },
        );
    }

    let mut badges = std::collections::BTreeMap::new();
    let mut badge_locks = std::collections::BTreeMap::new();
    for badge in &remote_badges {
        let name = badge.name.as_deref().unwrap_or("unnamed");
        let id = badge.id.unwrap_or(0);

        if badges.contains_key(name) {
            println!(
                "{} Duplicate badge name '{}' (id: {}) — skipping (only the first is kept)",
                "!".yellow(),
                name,
                id
            );
            continue;
        }

        let icon_asset_id = badge.icon_image_id;
        let (icon_path, icon_hash) = download_icon(
            &client,
            &icons_config,
            config_dir,
            "badge",
            id,
            name,
            &icon_asset_id,
        )
        .await?;

        badges.insert(
            name.to_string(),
            BadgeConfig {
                name: None,
                description: badge.description.clone(),
                icon: icon_path,
                enabled: badge.enabled.unwrap_or(true),
                path: None,
            },
        );
        badge_locks.insert(
            name.to_string(),
            BadgeLock {
                id,
                name: name.to_string(),
                description: badge.description.clone(),
                enabled: badge.enabled.unwrap_or(true),
                icon_asset_id,
                icon_hash,
            },
        );
    }

    let mut products = std::collections::BTreeMap::new();
    let mut product_locks = std::collections::BTreeMap::new();
    for product in &remote_products {
        let name = product.name.as_deref().unwrap_or("unnamed");
        let id = product.id.unwrap_or(0);

        if products.contains_key(name) {
            println!(
                "{} Duplicate product name '{}' (id: {}) — skipping (only the first is kept)",
                "!".yellow(),
                name,
                id
            );
            continue;
        }

        let icon_asset_id = product.icon_image_asset_id;
        let (icon_path, icon_hash) = download_icon(
            &client,
            &icons_config,
            config_dir,
            "product",
            id,
            name,
            &icon_asset_id,
        )
        .await?;

        let is_for_sale = product.is_for_sale.unwrap_or(true);
        let store_page = product.store_page_enabled.unwrap_or(false);
        products.insert(
            name.to_string(),
            ProductConfig {
                name: None,
                price: product.price().unwrap_or(0),
                description: product.description.clone(),
                icon: icon_path,
                for_sale: is_for_sale,
                regional_pricing: false,
                store_page,
                path: None,
            },
        );
        product_locks.insert(
            name.to_string(),
            ProductLock {
                id,
                name: name.to_string(),
                price: product.price().unwrap_or(0),
                description: product.description.clone(),
                icon_asset_id,
                icon_hash,
                for_sale: is_for_sale,
                regional_pricing: false,
                store_page,
            },
        );
    }

    let config = Config {
        experience: Experience {
            universe_id,
            creator: Creator {
                creator_type: CreatorType::User,
                id: 0,
            },
        },
        codegen: CodegenConfig::default(),
        icons: icons_config,
        passes,
        badges,
        products,
    };

    config.save(config_path)?;

    let lockfile_path = config_dir.join(LOCKFILE_NAME);
    let lockfile = Lockfile {
        version: 1,
        universe_id,
        passes: pass_locks,
        badges: badge_locks,
        products: product_locks,
    };
    lockfile.save(&lockfile_path)?;

    println!(
        "{} Created {} with {} passes, {} badges, {} products",
        "✓".green(),
        config_path.display(),
        config.passes.len(),
        config.badges.len(),
        config.products.len(),
    );
    println!("{} Created {}", "✓".green(), lockfile_path.display());

    Ok(())
}

/// Download an icon during init --from-remote.
/// Returns (relative icon path for config, icon hash for lockfile).
async fn download_icon(
    client: &RbxClient,
    icons_config: &IconsConfig,
    config_dir: &Path,
    resource_type: &str,
    resource_id: u64,
    name: &str,
    icon_asset_id: &Option<u64>,
) -> Result<(Option<std::path::PathBuf>, Option<String>)> {
    let Some(&asset_id) = icon_asset_id.as_ref() else {
        return Ok((None, None));
    };

    let relative_str = format!(
        "{}/{}-{}-{}.png",
        icons_config.dir.display(),
        resource_type,
        resource_id,
        name
    );
    let relative = PathBuf::from(&relative_str);
    let full_path = config_dir.join(&relative);

    println!(
        "  {} Downloading {} '{}' icon...",
        "↓".cyan(),
        resource_type,
        name
    );

    let bytes = client.download_asset(asset_id).await?;
    if let Some(parent) = full_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(&full_path, &bytes)?;
    let hash = blake3::hash(&bytes).to_hex().to_string();

    println!("  {} Saved to {}", "✓".green(), relative.display());

    Ok((Some(relative), Some(hash)))
}
