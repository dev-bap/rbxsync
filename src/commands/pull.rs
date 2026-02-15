use std::collections::HashMap;
use std::path::{Path, PathBuf};

use anyhow::{bail, Result};
use colored::Colorize;

use crate::api::RbxClient;
use crate::cli::Cli;
use crate::config::{BadgeConfig, Config, PassConfig, ProductConfig};
use crate::lockfile::{BadgeLock, Lockfile, PassLock, ProductLock, LOCKFILE_NAME};

struct IconConflict {
    resource_type: &'static str,
    name: String,
    local_path: String,
    local_hash: String,
    remote_asset_id: String,
}

struct PendingDownload {
    resource_type: &'static str,
    name: String,
    asset_id: u64,
    save_path: PathBuf,
}

struct ConfigChange {
    key: String,
    is_new: bool,
    field_changes: Vec<String>,
}

pub async fn run(cli: &Cli, dry_run: bool, accept_remote: bool, accept_local: bool) -> Result<()> {
    let mut config = Config::load(&cli.config)?;
    let config_dir = cli.config.parent().unwrap_or(Path::new("."));
    let lockfile_path = config_dir.join(LOCKFILE_NAME);

    // Load existing lockfile (if any) for conflict detection
    let old_lockfile = Lockfile::load(&lockfile_path)?;

    // Build ID → local key indexes from existing lockfile
    let pass_id_to_key: HashMap<u64, String> = old_lockfile
        .passes
        .iter()
        .map(|(k, v)| (v.id, k.clone()))
        .collect();
    let badge_id_to_key: HashMap<u64, String> = old_lockfile
        .badges
        .iter()
        .map(|(k, v)| (v.id, k.clone()))
        .collect();
    let product_id_to_key: HashMap<u64, String> = old_lockfile
        .products
        .iter()
        .map(|(k, v)| (v.id, k.clone()))
        .collect();

    let client = RbxClient::new(
        cli.api_key.clone(),
        config.experience.universe_id,
        config.icons.bleed,
    );

    println!("Pulling remote state...");

    // Fetch passes
    let remote_passes = client.list_all_game_passes().await?;
    let mut pass_locks = std::collections::BTreeMap::new();
    for pass in &remote_passes {
        let display_name = pass.name.as_deref().unwrap_or("unnamed");
        let id = pass.id.unwrap_or(0);
        let key = pass_id_to_key
            .get(&id)
            .cloned()
            .unwrap_or_else(|| display_name.to_string());

        if pass_locks.contains_key(&key) {
            println!(
                "{} Duplicate pass name '{}' (id: {}) — skipping",
                "!".yellow(),
                display_name,
                id
            );
            continue;
        }

        let remote_icon_asset_id = pass.icon_asset_id;
        pass_locks.insert(
            key,
            PassLock {
                id,
                name: display_name.to_string(),
                price: pass.price(),
                description: pass.description.clone(),
                icon_asset_id: remote_icon_asset_id,
                icon_hash: None,
                for_sale: pass.is_for_sale.unwrap_or(true),
                regional_pricing: false,
            },
        );
    }

    // Fetch badges
    let remote_badges = client
        .list_all_badges(config.experience.universe_id)
        .await?;
    let mut badge_locks = std::collections::BTreeMap::new();
    let mut seen_badge_ids = std::collections::HashSet::new();
    for badge in &remote_badges {
        let display_name = badge.name.as_deref().unwrap_or("unnamed");
        let id = badge.id.unwrap_or(0);
        seen_badge_ids.insert(id);
        let key = badge_id_to_key
            .get(&id)
            .cloned()
            .unwrap_or_else(|| display_name.to_string());

        if badge_locks.contains_key(&key) {
            println!(
                "{} Duplicate badge name '{}' (id: {}) — skipping",
                "!".yellow(),
                display_name,
                id
            );
            continue;
        }

        let remote_icon_asset_id = badge.icon_image_id;
        badge_locks.insert(
            key,
            BadgeLock {
                id,
                name: display_name.to_string(),
                description: badge.description.clone(),
                enabled: badge.enabled.unwrap_or(true),
                icon_asset_id: remote_icon_asset_id,
                icon_hash: None,
            },
        );
    }

    // The list endpoint omits disabled badges since August 2024.
    // Fetch them individually by ID so they don't appear as "removed".
    // See: https://devforum.roblox.com/t/improvements-to-badge-privacy/3098281
    for (key, old_lock) in &old_lockfile.badges {
        if !seen_badge_ids.contains(&old_lock.id) {
            match client.get_badge(old_lock.id).await {
                Ok(badge) => {
                    let display_name = badge.name.as_deref().unwrap_or("unnamed");
                    let id = badge.id.unwrap_or(old_lock.id);
                    badge_locks.insert(
                        key.clone(),
                        BadgeLock {
                            id,
                            name: display_name.to_string(),
                            description: badge.description.clone(),
                            enabled: badge.enabled.unwrap_or(false),
                            icon_asset_id: badge.icon_image_id,
                            icon_hash: None,
                        },
                    );
                }
                Err(_) => {
                    // Badge was truly deleted on Roblox — will show as "removed"
                }
            }
        }
    }

    // Fetch products
    let remote_products = client.list_all_developer_products().await?;
    let mut product_locks = std::collections::BTreeMap::new();
    for product in &remote_products {
        let display_name = product.name.as_deref().unwrap_or("unnamed");
        let id = product.id.unwrap_or(0);
        let key = product_id_to_key
            .get(&id)
            .cloned()
            .unwrap_or_else(|| display_name.to_string());

        if product_locks.contains_key(&key) {
            println!(
                "{} Duplicate product name '{}' (id: {}) — skipping",
                "!".yellow(),
                display_name,
                id
            );
            continue;
        }

        let remote_icon_asset_id = product.icon_image_asset_id;
        product_locks.insert(
            key,
            ProductLock {
                id,
                name: display_name.to_string(),
                price: product.price().unwrap_or(0),
                description: product.description.clone(),
                icon_asset_id: remote_icon_asset_id,
                icon_hash: None,
                for_sale: product.is_for_sale.unwrap_or(true),
                regional_pricing: false,
                store_page: product.store_page_enabled.unwrap_or(false),
            },
        );
    }

    // -----------------------------------------------------------------------
    // Update config from remote state
    // -----------------------------------------------------------------------
    let pass_config_changes = update_pass_config(&mut config, &pass_locks);
    let badge_config_changes = update_badge_config(&mut config, &badge_locks);
    let product_config_changes = update_product_config(&mut config, &product_locks);

    // -----------------------------------------------------------------------
    // Dry run — show diff and exit
    // -----------------------------------------------------------------------
    if dry_run {
        let mut has_diff = false;

        has_diff |= diff_passes(&old_lockfile, &pass_locks);
        has_diff |= diff_badges(&old_lockfile, &badge_locks);
        has_diff |= diff_products(&old_lockfile, &product_locks);

        let has_config_diff = print_config_changes("pass", &pass_config_changes)
            | print_config_changes("badge", &badge_config_changes)
            | print_config_changes("product", &product_config_changes);
        has_diff |= has_config_diff;

        if !has_diff {
            println!("{} Already up to date with remote.", "✓".green());
        } else {
            println!("\n{} Dry run — no changes applied.", "ℹ".blue());
        }
        return Ok(());
    }

    // -----------------------------------------------------------------------
    // Normal pull — detect icon conflicts, download, save lockfile
    // -----------------------------------------------------------------------

    // Detect icon conflicts
    let mut conflicts = Vec::new();
    let mut downloads: Vec<PendingDownload> = Vec::new();

    // Check passes
    for (name, new_lock) in &mut pass_locks {
        let old_icon_id = old_lockfile
            .passes
            .get(name)
            .and_then(|l| l.icon_asset_id.as_ref());
        let local_icon = config.passes.get(name).and_then(|c| c.icon.as_ref());

        match resolve_icon(
            "pass",
            name,
            new_lock.id,
            old_icon_id,
            &new_lock.icon_asset_id,
            local_icon,
            config_dir,
            &config.icons.dir,
            accept_remote,
            accept_local,
            &mut conflicts,
            &mut downloads,
        )? {
            IconResolution::SetNone => new_lock.icon_hash = None,
            IconResolution::PreserveOld => {
                new_lock.icon_hash = old_lockfile
                    .passes
                    .get(name)
                    .and_then(|l| l.icon_hash.clone());
            }
            IconResolution::PendingDownload => {}
        }
    }

    // Check badges
    for (name, new_lock) in &mut badge_locks {
        let old_icon_id = old_lockfile
            .badges
            .get(name)
            .and_then(|l| l.icon_asset_id.as_ref());
        let local_icon = config.badges.get(name).and_then(|c| c.icon.as_ref());

        match resolve_icon(
            "badge",
            name,
            new_lock.id,
            old_icon_id,
            &new_lock.icon_asset_id,
            local_icon,
            config_dir,
            &config.icons.dir,
            accept_remote,
            accept_local,
            &mut conflicts,
            &mut downloads,
        )? {
            IconResolution::SetNone => new_lock.icon_hash = None,
            IconResolution::PreserveOld => {
                new_lock.icon_hash = old_lockfile
                    .badges
                    .get(name)
                    .and_then(|l| l.icon_hash.clone());
            }
            IconResolution::PendingDownload => {}
        }
    }

    // Check products
    for (name, new_lock) in &mut product_locks {
        let old_icon_id = old_lockfile
            .products
            .get(name)
            .and_then(|l| l.icon_asset_id.as_ref());
        let local_icon = config.products.get(name).and_then(|c| c.icon.as_ref());

        match resolve_icon(
            "product",
            name,
            new_lock.id,
            old_icon_id,
            &new_lock.icon_asset_id,
            local_icon,
            config_dir,
            &config.icons.dir,
            accept_remote,
            accept_local,
            &mut conflicts,
            &mut downloads,
        )? {
            IconResolution::SetNone => new_lock.icon_hash = None,
            IconResolution::PreserveOld => {
                new_lock.icon_hash = old_lockfile
                    .products
                    .get(name)
                    .and_then(|l| l.icon_hash.clone());
            }
            IconResolution::PendingDownload => {}
        }
    }

    // If conflicts were collected (no flag provided), print and bail
    if !conflicts.is_empty() {
        println!();
        for c in &conflicts {
            println!(
                "{} {} '{}': icon differs from remote",
                "!".yellow(),
                c.resource_type,
                c.name.bold()
            );
            println!(
                "  Local:  {} (blake3: {}...)",
                c.local_path,
                &c.local_hash[..12]
            );
            println!("  Remote: asset {}", c.remote_asset_id);
        }
        println!();
        bail!(
            "Icon conflicts detected.\n  \
             Use --accept-remote to keep remote icons\n  \
             Use --accept-local to re-upload local icons on next sync"
        );
    }

    // Download remote icons (--accept-remote)
    for dl in &downloads {
        println!(
            "  {} Downloading {} '{}' icon...",
            "↓".cyan(),
            dl.resource_type,
            dl.name
        );
        let bytes = client.download_asset(dl.asset_id).await?;
        if let Some(parent) = dl.save_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(&dl.save_path, &bytes)?;
        let hash = hash_bytes(&bytes);

        // Compute path relative to config dir for config icon field
        let relative_icon = dl
            .save_path
            .strip_prefix(config_dir)
            .unwrap_or(&dl.save_path)
            .to_path_buf();

        match dl.resource_type {
            "pass" => {
                if let Some(lock) = pass_locks.get_mut(&dl.name) {
                    lock.icon_hash = Some(hash);
                }
                if let Some(pc) = config.passes.get_mut(&dl.name) {
                    if pc.icon.is_none() {
                        pc.icon = Some(relative_icon.clone());
                    }
                }
            }
            "badge" => {
                if let Some(lock) = badge_locks.get_mut(&dl.name) {
                    lock.icon_hash = Some(hash);
                }
                if let Some(bc) = config.badges.get_mut(&dl.name) {
                    if bc.icon.is_none() {
                        bc.icon = Some(relative_icon.clone());
                    }
                }
            }
            "product" => {
                if let Some(lock) = product_locks.get_mut(&dl.name) {
                    lock.icon_hash = Some(hash);
                }
                if let Some(pc) = config.products.get_mut(&dl.name) {
                    if pc.icon.is_none() {
                        pc.icon = Some(relative_icon.clone());
                    }
                }
            }
            _ => {}
        }

        println!("  {} Saved to {}", "✓".green(), dl.save_path.display());
    }

    let lockfile = Lockfile {
        version: 1,
        universe_id: config.experience.universe_id,
        passes: pass_locks,
        badges: badge_locks,
        products: product_locks,
    };

    lockfile.save(&lockfile_path)?;
    config.save(&cli.config)?;

    println!(
        "{} Updated: {} passes, {} badges, {} products",
        "✓".green(),
        lockfile.passes.len(),
        lockfile.badges.len(),
        lockfile.products.len(),
    );

    Ok(())
}

// ---------------------------------------------------------------------------
// Config update helpers
// ---------------------------------------------------------------------------

/// Convert lock name to config name field: None if name == key (convention).
fn config_name(lock_name: &str, key: &str) -> Option<String> {
    if lock_name == key {
        None
    } else {
        Some(lock_name.to_string())
    }
}

fn update_pass_config(
    config: &mut Config,
    pass_locks: &std::collections::BTreeMap<String, PassLock>,
) -> Vec<ConfigChange> {
    let mut changes = Vec::new();

    for (key, lock) in pass_locks {
        let new_name = config_name(&lock.name, key);
        if let Some(pc) = config.passes.get_mut(key) {
            // Existing entry — update remote-visible fields, preserve config-only
            let mut field_changes = Vec::new();
            if pc.name != new_name {
                field_changes.push(format!("name: {:?} -> {:?}", pc.name, new_name));
                pc.name = new_name;
            }
            if pc.price != lock.price {
                field_changes.push(format!("price: {:?} -> {:?}", pc.price, lock.price));
                pc.price = lock.price;
            }
            if pc.description != lock.description {
                field_changes.push(format!(
                    "description: {:?} -> {:?}",
                    pc.description, lock.description
                ));
                pc.description = lock.description.clone();
            }
            if pc.for_sale != lock.for_sale {
                field_changes.push(format!("for_sale: {} -> {}", pc.for_sale, lock.for_sale));
                pc.for_sale = lock.for_sale;
            }
            if !field_changes.is_empty() {
                changes.push(ConfigChange {
                    key: key.clone(),
                    is_new: false,
                    field_changes,
                });
            }
        } else {
            // New entry from remote
            config.passes.insert(
                key.clone(),
                PassConfig {
                    name: new_name,
                    price: lock.price,
                    description: lock.description.clone(),
                    icon: None,
                    for_sale: lock.for_sale,
                    regional_pricing: false,
                    path: None,
                },
            );
            changes.push(ConfigChange {
                key: key.clone(),
                is_new: true,
                field_changes: Vec::new(),
            });
        }
    }

    changes
}

fn update_badge_config(
    config: &mut Config,
    badge_locks: &std::collections::BTreeMap<String, BadgeLock>,
) -> Vec<ConfigChange> {
    let mut changes = Vec::new();

    for (key, lock) in badge_locks {
        let new_name = config_name(&lock.name, key);
        if let Some(bc) = config.badges.get_mut(key) {
            let mut field_changes = Vec::new();
            if bc.name != new_name {
                field_changes.push(format!("name: {:?} -> {:?}", bc.name, new_name));
                bc.name = new_name;
            }
            if bc.description != lock.description {
                field_changes.push(format!(
                    "description: {:?} -> {:?}",
                    bc.description, lock.description
                ));
                bc.description = lock.description.clone();
            }
            if bc.enabled != lock.enabled {
                field_changes.push(format!("enabled: {} -> {}", bc.enabled, lock.enabled));
                bc.enabled = lock.enabled;
            }
            if !field_changes.is_empty() {
                changes.push(ConfigChange {
                    key: key.clone(),
                    is_new: false,
                    field_changes,
                });
            }
        } else {
            config.badges.insert(
                key.clone(),
                BadgeConfig {
                    name: new_name,
                    description: lock.description.clone(),
                    icon: None,
                    enabled: lock.enabled,
                    path: None,
                },
            );
            changes.push(ConfigChange {
                key: key.clone(),
                is_new: true,
                field_changes: Vec::new(),
            });
        }
    }

    changes
}

fn update_product_config(
    config: &mut Config,
    product_locks: &std::collections::BTreeMap<String, ProductLock>,
) -> Vec<ConfigChange> {
    let mut changes = Vec::new();

    for (key, lock) in product_locks {
        let new_name = config_name(&lock.name, key);
        if let Some(pc) = config.products.get_mut(key) {
            let mut field_changes = Vec::new();
            if pc.name != new_name {
                field_changes.push(format!("name: {:?} -> {:?}", pc.name, new_name));
                pc.name = new_name;
            }
            if pc.price != lock.price {
                field_changes.push(format!("price: {} -> {}", pc.price, lock.price));
                pc.price = lock.price;
            }
            if pc.description != lock.description {
                field_changes.push(format!(
                    "description: {:?} -> {:?}",
                    pc.description, lock.description
                ));
                pc.description = lock.description.clone();
            }
            if pc.for_sale != lock.for_sale {
                field_changes.push(format!("for_sale: {} -> {}", pc.for_sale, lock.for_sale));
                pc.for_sale = lock.for_sale;
            }
            if pc.store_page != lock.store_page {
                field_changes.push(format!(
                    "store_page: {} -> {}",
                    pc.store_page, lock.store_page
                ));
                pc.store_page = lock.store_page;
            }
            if !field_changes.is_empty() {
                changes.push(ConfigChange {
                    key: key.clone(),
                    is_new: false,
                    field_changes,
                });
            }
        } else {
            config.products.insert(
                key.clone(),
                ProductConfig {
                    name: new_name,
                    price: lock.price,
                    description: lock.description.clone(),
                    icon: None,
                    for_sale: lock.for_sale,
                    regional_pricing: false,
                    store_page: lock.store_page,
                    path: None,
                },
            );
            changes.push(ConfigChange {
                key: key.clone(),
                is_new: true,
                field_changes: Vec::new(),
            });
        }
    }

    changes
}

fn print_config_changes(resource_type: &str, changes: &[ConfigChange]) -> bool {
    let mut has_diff = false;
    for change in changes {
        has_diff = true;
        if change.is_new {
            println!(
                "  {} {} {} {} in config",
                "+".green(),
                "add".green(),
                resource_type,
                change.key.bold()
            );
        } else {
            println!(
                "  {} {} {} {} in config",
                "~".yellow(),
                "update".yellow(),
                resource_type,
                change.key.bold()
            );
            for fc in &change.field_changes {
                println!("    {} {}", "·".dimmed(), fc);
            }
        }
    }
    has_diff
}

// ---------------------------------------------------------------------------
// Dry-run diff helpers
// ---------------------------------------------------------------------------

fn diff_passes(old: &Lockfile, remote: &std::collections::BTreeMap<String, PassLock>) -> bool {
    let mut has_diff = false;

    for (key, new_lock) in remote {
        if let Some(old_lock) = old.passes.get(key) {
            let mut changes = Vec::new();
            if old_lock.name != new_lock.name {
                changes.push(format!("name: {} -> {}", old_lock.name, new_lock.name));
            }
            if old_lock.price != new_lock.price {
                changes.push(format!(
                    "price: {:?} -> {:?}",
                    old_lock.price, new_lock.price
                ));
            }
            if old_lock.description != new_lock.description {
                changes.push(format!(
                    "description: {:?} -> {:?}",
                    old_lock.description.as_deref().unwrap_or(""),
                    new_lock.description.as_deref().unwrap_or("")
                ));
            }
            if old_lock.for_sale != new_lock.for_sale {
                changes.push(format!(
                    "for_sale: {} -> {}",
                    old_lock.for_sale, new_lock.for_sale
                ));
            }
            if old_lock.icon_asset_id != new_lock.icon_asset_id {
                changes.push(format!(
                    "icon: {:?} -> {:?}",
                    old_lock.icon_asset_id, new_lock.icon_asset_id
                ));
            }
            if !changes.is_empty() {
                has_diff = true;
                println!(
                    "  {} {} pass {}",
                    "~".yellow(),
                    "update".yellow(),
                    key.bold()
                );
                for c in &changes {
                    println!("    {} {}", "·".dimmed(), c);
                }
            }
        } else {
            has_diff = true;
            println!(
                "  {} {} pass {} (id: {})",
                "+".green(),
                "new".green(),
                key.bold(),
                new_lock.id
            );
        }
    }

    for key in old.passes.keys() {
        if !remote.contains_key(key) {
            has_diff = true;
            println!("  {} {} pass {}", "-".red(), "removed".red(), key.bold());
        }
    }

    has_diff
}

fn diff_badges(old: &Lockfile, remote: &std::collections::BTreeMap<String, BadgeLock>) -> bool {
    let mut has_diff = false;

    for (key, new_lock) in remote {
        if let Some(old_lock) = old.badges.get(key) {
            let mut changes = Vec::new();
            if old_lock.name != new_lock.name {
                changes.push(format!("name: {} -> {}", old_lock.name, new_lock.name));
            }
            if old_lock.description != new_lock.description {
                changes.push(format!(
                    "description: {:?} -> {:?}",
                    old_lock.description.as_deref().unwrap_or(""),
                    new_lock.description.as_deref().unwrap_or("")
                ));
            }
            if old_lock.enabled != new_lock.enabled {
                changes.push(format!(
                    "enabled: {} -> {}",
                    old_lock.enabled, new_lock.enabled
                ));
            }
            if old_lock.icon_asset_id != new_lock.icon_asset_id {
                changes.push(format!(
                    "icon: {:?} -> {:?}",
                    old_lock.icon_asset_id, new_lock.icon_asset_id
                ));
            }
            if !changes.is_empty() {
                has_diff = true;
                println!(
                    "  {} {} badge {}",
                    "~".yellow(),
                    "update".yellow(),
                    key.bold()
                );
                for c in &changes {
                    println!("    {} {}", "·".dimmed(), c);
                }
            }
        } else {
            has_diff = true;
            println!(
                "  {} {} badge {} (id: {})",
                "+".green(),
                "new".green(),
                key.bold(),
                new_lock.id
            );
        }
    }

    for key in old.badges.keys() {
        if !remote.contains_key(key) {
            has_diff = true;
            println!("  {} {} badge {}", "-".red(), "removed".red(), key.bold());
        }
    }

    has_diff
}

fn diff_products(old: &Lockfile, remote: &std::collections::BTreeMap<String, ProductLock>) -> bool {
    let mut has_diff = false;

    for (key, new_lock) in remote {
        if let Some(old_lock) = old.products.get(key) {
            let mut changes = Vec::new();
            if old_lock.name != new_lock.name {
                changes.push(format!("name: {} -> {}", old_lock.name, new_lock.name));
            }
            if old_lock.price != new_lock.price {
                changes.push(format!("price: {} -> {}", old_lock.price, new_lock.price));
            }
            if old_lock.description != new_lock.description {
                changes.push(format!(
                    "description: {:?} -> {:?}",
                    old_lock.description.as_deref().unwrap_or(""),
                    new_lock.description.as_deref().unwrap_or("")
                ));
            }
            if old_lock.for_sale != new_lock.for_sale {
                changes.push(format!(
                    "for_sale: {} -> {}",
                    old_lock.for_sale, new_lock.for_sale
                ));
            }
            if old_lock.store_page != new_lock.store_page {
                changes.push(format!(
                    "store_page: {} -> {}",
                    old_lock.store_page, new_lock.store_page
                ));
            }
            if old_lock.icon_asset_id != new_lock.icon_asset_id {
                changes.push(format!(
                    "icon: {:?} -> {:?}",
                    old_lock.icon_asset_id, new_lock.icon_asset_id
                ));
            }
            if !changes.is_empty() {
                has_diff = true;
                println!(
                    "  {} {} product {}",
                    "~".yellow(),
                    "update".yellow(),
                    key.bold()
                );
                for c in &changes {
                    println!("    {} {}", "·".dimmed(), c);
                }
            }
        } else {
            has_diff = true;
            println!(
                "  {} {} product {} (id: {})",
                "+".green(),
                "new".green(),
                key.bold(),
                new_lock.id
            );
        }
    }

    for key in old.products.keys() {
        if !remote.contains_key(key) {
            has_diff = true;
            println!("  {} {} product {}", "-".red(), "removed".red(), key.bold());
        }
    }

    has_diff
}

// ---------------------------------------------------------------------------
// Icon resolution
// ---------------------------------------------------------------------------

enum IconResolution {
    /// Set icon_hash to None
    SetNone,
    /// Preserve the old lockfile's icon_hash
    PreserveOld,
    /// Download the remote icon (hash set after download)
    PendingDownload,
}

/// Determine how to handle icon_hash for a resource during pull.
///
/// Returns the resolution and optionally appends to `conflicts` or `downloads`.
#[allow(clippy::too_many_arguments)]
fn resolve_icon(
    resource_type: &'static str,
    name: &str,
    resource_id: u64,
    old_icon_id: Option<&u64>,
    new_icon_id: &Option<u64>,
    local_icon: Option<&std::path::PathBuf>,
    config_dir: &Path,
    icon_dir: &Path,
    accept_remote: bool,
    accept_local: bool,
    conflicts: &mut Vec<IconConflict>,
    downloads: &mut Vec<PendingDownload>,
) -> Result<IconResolution> {
    let icon_changed = match (old_icon_id, new_icon_id.as_ref()) {
        (Some(old), Some(new)) => old != new,
        (Some(_), None) | (None, Some(_)) => true,
        (None, None) => false,
    };

    if !icon_changed {
        // No change in remote icon_asset_id — preserve old hash
        return Ok(IconResolution::PreserveOld);
    }

    // Icon changed on remote
    if accept_remote {
        if let Some(&asset_id) = new_icon_id.as_ref() {
            let save_path = if let Some(local_path) = local_icon {
                config_dir.join(local_path)
            } else {
                config_dir.join(format!(
                    "{}/{}-{}-{}.png",
                    icon_dir.display(),
                    resource_type,
                    resource_id,
                    name
                ))
            };
            downloads.push(PendingDownload {
                resource_type,
                name: name.to_string(),
                asset_id,
                save_path,
            });
            return Ok(IconResolution::PendingDownload);
        } else {
            // Remote icon was removed
            return Ok(IconResolution::SetNone);
        }
    }

    if accept_local {
        // Re-upload local icon on next sync
        return Ok(IconResolution::SetNone);
    }

    // No flag — check for conflict
    let Some(local_path) = local_icon else {
        // No local icon configured — no conflict, just clear hash
        return Ok(IconResolution::SetNone);
    };

    let full_path = config_dir.join(local_path);
    let local_hash = hash_file(&full_path)?;

    conflicts.push(IconConflict {
        resource_type,
        name: name.to_string(),
        local_path: local_path.display().to_string(),
        local_hash,
        remote_asset_id: new_icon_id
            .map(|id| id.to_string())
            .unwrap_or_else(|| "none".to_string()),
    });
    // Temporarily return SetNone — won't be used since we'll bail
    Ok(IconResolution::SetNone)
}

fn hash_file(path: &Path) -> Result<String> {
    let bytes = std::fs::read(path)?;
    Ok(blake3::hash(&bytes).to_hex().to_string())
}

fn hash_bytes(bytes: &[u8]) -> String {
    blake3::hash(bytes).to_hex().to_string()
}
