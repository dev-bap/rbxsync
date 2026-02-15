use std::path::Path;

use anyhow::Result;
use colored::Colorize;

use crate::api::RbxClient;
use crate::cli::{Cli, ResourceType};
use crate::codegen;
use crate::config::{resolve_name, Config};
use crate::diff::{build_sync_plan, Action};
use crate::lockfile::{BadgeLock, Lockfile, PassLock, ProductLock};

pub async fn run(
    cli: &Cli,
    dry_run: bool,
    only: Option<Vec<ResourceType>>,
    badge_cost: u64,
) -> Result<()> {
    let config = Config::load(&cli.config)?;
    let config_dir = cli.config.parent().unwrap_or(Path::new("."));
    let lockfile_path = config_dir.join(crate::lockfile::LOCKFILE_NAME);
    let mut lockfile = Lockfile::load(&lockfile_path)?;
    lockfile.universe_id = config.experience.universe_id;
    lockfile.version = 1;

    let plan = build_sync_plan(&config, &lockfile, config_dir)?;

    for warning in &plan.warnings {
        println!("{} {}", "!".yellow(), warning);
    }

    if !plan.has_changes() {
        println!("{} Everything is up to date.", "✓".green());

        if let Some(output) = &config.codegen.output {
            let output_path = config_dir.join(output);
            let tree = codegen::build_tree(&lockfile, &config);
            codegen::generate_luau(&tree, &output_path)?;
            println!("{} Generated {}", "✓".green(), output_path.display());

            if config.codegen.typescript {
                let ts_path = output_path.with_extension("d.ts");
                codegen::generate_typescript(&tree, &ts_path)?;
                println!("{} Generated {}", "✓".green(), ts_path.display());
            }
        }

        return Ok(());
    }

    let should_sync =
        |rt: &ResourceType| -> bool { only.as_ref().is_none_or(|types| types.contains(rt)) };

    if should_sync(&ResourceType::Passes) {
        for action in &plan.passes {
            print_action("pass", action);
        }
    }
    if should_sync(&ResourceType::Badges) {
        for action in &plan.badges {
            print_action("badge", action);
        }
    }
    if should_sync(&ResourceType::Products) {
        for action in &plan.products {
            print_action("product", action);
        }
    }

    println!("\n{}", plan.summary());

    if dry_run {
        println!("\n{} Dry run — no changes applied.", "ℹ".blue());
        return Ok(());
    }

    let client = RbxClient::new(
        cli.api_key.clone(),
        config.experience.universe_id,
        config.icons.bleed,
    );

    // Sync passes
    if should_sync(&ResourceType::Passes) {
        for action in &plan.passes {
            match &action.action {
                Action::Create => {
                    let pass_cfg = &config.passes[&action.name];
                    let display_name = resolve_name(pass_cfg.name.as_deref(), &action.name);
                    let icon_path = pass_cfg.icon.as_ref().map(|p| config_dir.join(p));
                    let icon_hash = icon_path.as_ref().map(|p| hash_file(p)).transpose()?;

                    print!("  Creating pass '{}'...", action.name);
                    let result = client
                        .create_game_pass(
                            display_name,
                            pass_cfg.description.as_deref(),
                            pass_cfg.price,
                            icon_path.as_deref(),
                            pass_cfg.for_sale,
                            pass_cfg.regional_pricing,
                        )
                        .await?;

                    let id = result.id.unwrap_or(0);
                    println!(" {} (id: {})", "done".green(), id);

                    lockfile.passes.insert(
                        action.name.clone(),
                        PassLock {
                            id,
                            name: display_name.to_string(),
                            price: pass_cfg.price,
                            description: pass_cfg.description.clone(),
                            icon_asset_id: result.icon_asset_id,
                            icon_hash,
                            for_sale: pass_cfg.for_sale,
                            regional_pricing: pass_cfg.regional_pricing,
                        },
                    );
                    lockfile.save(&lockfile_path)?;
                }
                Action::Update { .. } => {
                    let pass_cfg = &config.passes[&action.name];
                    let display_name = resolve_name(pass_cfg.name.as_deref(), &action.name);
                    let lock = &lockfile.passes[&action.name];
                    let icon_path = pass_cfg.icon.as_ref().map(|p| config_dir.join(p));
                    let icon_hash = icon_path.as_ref().map(|p| hash_file(p)).transpose()?;

                    let icon_changed = match (&icon_hash, &lock.icon_hash) {
                        (Some(new), Some(old)) => new != old,
                        (Some(_), None) => true,
                        _ => false,
                    };
                    let send_icon = if icon_changed {
                        icon_path.as_deref()
                    } else {
                        None
                    };

                    print!("  Updating pass '{}'...", action.name);
                    let result = client
                        .update_game_pass(
                            lock.id,
                            display_name,
                            pass_cfg.description.as_deref(),
                            pass_cfg.price,
                            send_icon,
                            pass_cfg.for_sale,
                            pass_cfg.regional_pricing,
                        )
                        .await?;
                    println!(" {}", "done".green());

                    lockfile.passes.insert(
                        action.name.clone(),
                        PassLock {
                            id: lock.id,
                            name: display_name.to_string(),
                            price: pass_cfg.price,
                            description: pass_cfg.description.clone(),
                            icon_asset_id: result.icon_asset_id.or(lock.icon_asset_id),
                            icon_hash: icon_hash.or(lock.icon_hash.clone()),
                            for_sale: pass_cfg.for_sale,
                            regional_pricing: pass_cfg.regional_pricing,
                        },
                    );
                    lockfile.save(&lockfile_path)?;
                }
                Action::Skip => {}
            }
        }
    }

    // Sync badges
    if should_sync(&ResourceType::Badges) {
        let payment_source: u32 = match &config.experience.creator.creator_type {
            crate::config::CreatorType::User => 1,
            crate::config::CreatorType::Group => 2,
        };

        for action in &plan.badges {
            match &action.action {
                Action::Create => {
                    let badge_cfg = &config.badges[&action.name];
                    let display_name = resolve_name(badge_cfg.name.as_deref(), &action.name);
                    let icon_path = badge_cfg.icon.as_ref().map(|p| config_dir.join(p));
                    let icon_hash = icon_path.as_ref().map(|p| hash_file(p)).transpose()?;

                    print!("  Creating badge '{}'...", action.name);
                    let result = client
                        .create_badge(
                            display_name,
                            badge_cfg.description.as_deref(),
                            icon_path.as_deref(),
                            payment_source,
                            badge_cost,
                        )
                        .await?;

                    let id = result.id.unwrap_or(0);
                    println!(" {} (id: {})", "done".green(), id);

                    lockfile.badges.insert(
                        action.name.clone(),
                        BadgeLock {
                            id,
                            name: display_name.to_string(),
                            description: badge_cfg.description.clone(),
                            enabled: badge_cfg.enabled,
                            icon_asset_id: result.icon_image_id,
                            icon_hash,
                        },
                    );
                    lockfile.save(&lockfile_path)?;
                }
                Action::Update { changes } => {
                    let badge_cfg = &config.badges[&action.name];
                    let display_name = resolve_name(badge_cfg.name.as_deref(), &action.name);
                    let lock = &lockfile.badges[&action.name];

                    let icon_changed = changes.iter().any(|c| c.field == "icon");
                    let has_metadata_changes = changes.iter().any(|c| c.field != "icon");

                    if has_metadata_changes {
                        print!("  Updating badge '{}'...", action.name);
                        client
                            .update_badge(
                                lock.id,
                                display_name,
                                badge_cfg.description.as_deref(),
                                badge_cfg.enabled,
                            )
                            .await?;
                        println!(" {}", "done".green());
                    }

                    let new_icon_asset_id = lock.icon_asset_id;
                    let mut new_icon_hash = lock.icon_hash.clone();

                    if icon_changed {
                        if let Some(icon) = &badge_cfg.icon {
                            let icon_path = config_dir.join(icon);
                            print!("  Updating badge '{}' icon...", action.name);
                            let icon_result = client.update_badge_icon(lock.id, &icon_path).await?;
                            new_icon_hash = Some(hash_file(&icon_path)?);
                            println!(" {}", "done".green());
                            // Update icon_asset_id if returned
                            let final_icon_id = icon_result.target_id.or(new_icon_asset_id);

                            lockfile.badges.insert(
                                action.name.clone(),
                                BadgeLock {
                                    id: lock.id,
                                    name: display_name.to_string(),
                                    description: badge_cfg.description.clone(),
                                    enabled: badge_cfg.enabled,
                                    icon_asset_id: final_icon_id,
                                    icon_hash: new_icon_hash,
                                },
                            );
                            lockfile.save(&lockfile_path)?;
                            continue;
                        }
                    }

                    lockfile.badges.insert(
                        action.name.clone(),
                        BadgeLock {
                            id: lock.id,
                            name: display_name.to_string(),
                            description: badge_cfg.description.clone(),
                            enabled: badge_cfg.enabled,
                            icon_asset_id: new_icon_asset_id,
                            icon_hash: new_icon_hash,
                        },
                    );
                    lockfile.save(&lockfile_path)?;
                }
                Action::Skip => {}
            }
        }
    }

    // Sync products
    if should_sync(&ResourceType::Products) {
        for action in &plan.products {
            match &action.action {
                Action::Create => {
                    let product_cfg = &config.products[&action.name];
                    let display_name = resolve_name(product_cfg.name.as_deref(), &action.name);
                    let icon_path = product_cfg.icon.as_ref().map(|p| config_dir.join(p));
                    let icon_hash = icon_path.as_ref().map(|p| hash_file(p)).transpose()?;

                    print!("  Creating product '{}'...", action.name);
                    let result = client
                        .create_developer_product(
                            display_name,
                            product_cfg.description.as_deref(),
                            product_cfg.price,
                            icon_path.as_deref(),
                            product_cfg.for_sale,
                            product_cfg.regional_pricing,
                        )
                        .await?;

                    let id = result.id.unwrap_or(0);
                    println!(" {} (id: {})", "done".green(), id);

                    lockfile.products.insert(
                        action.name.clone(),
                        ProductLock {
                            id,
                            name: display_name.to_string(),
                            price: product_cfg.price,
                            description: product_cfg.description.clone(),
                            icon_asset_id: result.icon_image_asset_id,
                            icon_hash,
                            for_sale: product_cfg.for_sale,
                            regional_pricing: product_cfg.regional_pricing,
                            store_page: product_cfg.store_page,
                        },
                    );
                    lockfile.save(&lockfile_path)?;
                }
                Action::Update { .. } => {
                    let product_cfg = &config.products[&action.name];
                    let display_name = resolve_name(product_cfg.name.as_deref(), &action.name);
                    let lock = &lockfile.products[&action.name];
                    let icon_path = product_cfg.icon.as_ref().map(|p| config_dir.join(p));
                    let icon_hash = icon_path.as_ref().map(|p| hash_file(p)).transpose()?;

                    let icon_changed = match (&icon_hash, &lock.icon_hash) {
                        (Some(new), Some(old)) => new != old,
                        (Some(_), None) => true,
                        _ => false,
                    };
                    let send_icon = if icon_changed {
                        icon_path.as_deref()
                    } else {
                        None
                    };

                    print!("  Updating product '{}'...", action.name);
                    let result = client
                        .update_developer_product(
                            lock.id,
                            display_name,
                            product_cfg.description.as_deref(),
                            product_cfg.price,
                            send_icon,
                            product_cfg.for_sale,
                            product_cfg.regional_pricing,
                            product_cfg.store_page,
                        )
                        .await?;
                    println!(" {}", "done".green());

                    lockfile.products.insert(
                        action.name.clone(),
                        ProductLock {
                            id: lock.id,
                            name: display_name.to_string(),
                            price: product_cfg.price,
                            description: product_cfg.description.clone(),
                            icon_asset_id: result.icon_image_asset_id.or(lock.icon_asset_id),
                            icon_hash: icon_hash.or(lock.icon_hash.clone()),
                            for_sale: product_cfg.for_sale,
                            regional_pricing: product_cfg.regional_pricing,
                            store_page: product_cfg.store_page,
                        },
                    );
                    lockfile.save(&lockfile_path)?;
                }
                Action::Skip => {}
            }
        }
    }

    println!("{} Sync complete.", "✓".green());

    if let Some(output) = &config.codegen.output {
        let output_path = config_dir.join(output);
        let tree = codegen::build_tree(&lockfile, &config);
        codegen::generate_luau(&tree, &output_path)?;
        println!("{} Generated {}", "✓".green(), output_path.display());

        if config.codegen.typescript {
            let ts_path = output_path.with_extension("d.ts");
            codegen::generate_typescript(&tree, &ts_path)?;
            println!("{} Generated {}", "✓".green(), ts_path.display());
        }
    }

    Ok(())
}

fn print_action(resource_type: &str, action: &crate::diff::ResourceAction) {
    match &action.action {
        Action::Create => {
            println!(
                "  {} {} {} {}",
                "+".green(),
                "create".green(),
                resource_type,
                action.name.bold()
            );
        }
        Action::Update { changes } => {
            println!(
                "  {} {} {} {}",
                "~".yellow(),
                "update".yellow(),
                resource_type,
                action.name.bold()
            );
            for change in changes {
                println!("    {} {}", "·".dimmed(), change);
            }
        }
        Action::Skip => {
            println!(
                "  {} {} {} {}",
                "=".dimmed(),
                "skip".dimmed(),
                resource_type,
                action.name.dimmed()
            );
        }
    }
}

fn hash_file(path: &Path) -> Result<String> {
    let bytes = std::fs::read(path)?;
    Ok(blake3::hash(&bytes).to_hex().to_string())
}
