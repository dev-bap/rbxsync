use std::fmt;
use std::path::Path;

use anyhow::Result;

use crate::config::{resolve_name, Config};
use crate::lockfile::Lockfile;

#[derive(Debug)]
pub struct SyncPlan {
    pub passes: Vec<ResourceAction>,
    pub badges: Vec<ResourceAction>,
    pub products: Vec<ResourceAction>,
    pub warnings: Vec<String>,
}

#[derive(Debug)]
pub struct ResourceAction {
    pub name: String,
    pub action: Action,
}

#[derive(Debug)]
pub enum Action {
    Create,
    Update { changes: Vec<FieldChange> },
    Skip,
}

#[derive(Debug)]
pub struct FieldChange {
    pub field: String,
    pub old: String,
    pub new: String,
}

impl fmt::Display for FieldChange {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {} -> {}", self.field, self.old, self.new)
    }
}

impl SyncPlan {
    pub fn has_changes(&self) -> bool {
        self.passes
            .iter()
            .chain(&self.badges)
            .chain(&self.products)
            .any(|a| !matches!(a.action, Action::Skip))
    }

    pub fn summary(&self) -> String {
        let mut creates = 0;
        let mut updates = 0;
        let mut skips = 0;

        for action in self.passes.iter().chain(&self.badges).chain(&self.products) {
            match &action.action {
                Action::Create => creates += 1,
                Action::Update { .. } => updates += 1,
                Action::Skip => skips += 1,
            }
        }

        format!(
            "{} to create, {} to update, {} unchanged",
            creates, updates, skips
        )
    }
}

fn hash_file(path: &Path) -> Result<String> {
    let bytes = std::fs::read(path)?;
    Ok(blake3::hash(&bytes).to_hex().to_string())
}

pub fn build_sync_plan(
    config: &Config,
    lockfile: &Lockfile,
    config_dir: &Path,
) -> Result<SyncPlan> {
    let mut warnings = Vec::new();

    // Check for resources in lockfile but not in config
    for key in lockfile.passes.keys() {
        if !config.passes.contains_key(key) {
            warnings.push(format!(
                "Pass '{}' exists in lockfile but not in config (will not be deleted)",
                key
            ));
        }
    }
    for key in lockfile.badges.keys() {
        if !config.badges.contains_key(key) {
            warnings.push(format!(
                "Badge '{}' exists in lockfile but not in config (will not be deleted)",
                key
            ));
        }
    }
    for key in lockfile.products.keys() {
        if !config.products.contains_key(key) {
            warnings.push(format!(
                "Product '{}' exists in lockfile but not in config (will not be deleted)",
                key
            ));
        }
    }

    // Diff passes
    let passes = diff_passes(config, lockfile, config_dir)?;
    let badges = diff_badges(config, lockfile, config_dir)?;
    let products = diff_products(config, lockfile, config_dir)?;

    Ok(SyncPlan {
        passes,
        badges,
        products,
        warnings,
    })
}

fn diff_passes(
    config: &Config,
    lockfile: &Lockfile,
    config_dir: &Path,
) -> Result<Vec<ResourceAction>> {
    let mut actions = Vec::new();

    for (name, pass_cfg) in &config.passes {
        match lockfile.passes.get(name) {
            None => {
                actions.push(ResourceAction {
                    name: name.clone(),
                    action: Action::Create,
                });
            }
            Some(lock) => {
                let mut changes = Vec::new();

                let cfg_name = resolve_name(pass_cfg.name.as_deref(), name);
                if cfg_name != lock.name {
                    changes.push(FieldChange {
                        field: "name".to_string(),
                        old: lock.name.clone(),
                        new: cfg_name.to_string(),
                    });
                }
                if pass_cfg.price != lock.price {
                    changes.push(FieldChange {
                        field: "price".to_string(),
                        old: format!("{:?}", lock.price),
                        new: format!("{:?}", pass_cfg.price),
                    });
                }
                let cfg_desc = pass_cfg.description.as_deref().unwrap_or("");
                let lock_desc = lock.description.as_deref().unwrap_or("");
                if cfg_desc != lock_desc {
                    changes.push(FieldChange {
                        field: "description".to_string(),
                        old: lock_desc.to_string(),
                        new: cfg_desc.to_string(),
                    });
                }
                if pass_cfg.for_sale != lock.for_sale {
                    changes.push(FieldChange {
                        field: "for_sale".to_string(),
                        old: lock.for_sale.to_string(),
                        new: pass_cfg.for_sale.to_string(),
                    });
                }
                if pass_cfg.regional_pricing != lock.regional_pricing {
                    changes.push(FieldChange {
                        field: "regional_pricing".to_string(),
                        old: lock.regional_pricing.to_string(),
                        new: pass_cfg.regional_pricing.to_string(),
                    });
                }

                if let Some(icon) = &pass_cfg.icon {
                    let full_path = config_dir.join(icon);
                    let current_hash = hash_file(&full_path)?;
                    let lock_hash = lock.icon_hash.as_deref().unwrap_or("");
                    if current_hash != lock_hash {
                        changes.push(FieldChange {
                            field: "icon".to_string(),
                            old: lock_hash.chars().take(8).collect::<String>() + "...",
                            new: current_hash.chars().take(8).collect::<String>() + "...",
                        });
                    }
                }

                if changes.is_empty() {
                    actions.push(ResourceAction {
                        name: name.clone(),
                        action: Action::Skip,
                    });
                } else {
                    actions.push(ResourceAction {
                        name: name.clone(),
                        action: Action::Update { changes },
                    });
                }
            }
        }
    }

    Ok(actions)
}

fn diff_badges(
    config: &Config,
    lockfile: &Lockfile,
    config_dir: &Path,
) -> Result<Vec<ResourceAction>> {
    let mut actions = Vec::new();

    for (name, badge_cfg) in &config.badges {
        match lockfile.badges.get(name) {
            None => {
                actions.push(ResourceAction {
                    name: name.clone(),
                    action: Action::Create,
                });
            }
            Some(lock) => {
                let mut changes = Vec::new();

                let cfg_name = resolve_name(badge_cfg.name.as_deref(), name);
                if cfg_name != lock.name {
                    changes.push(FieldChange {
                        field: "name".to_string(),
                        old: lock.name.clone(),
                        new: cfg_name.to_string(),
                    });
                }
                let cfg_desc = badge_cfg.description.as_deref().unwrap_or("");
                let lock_desc = lock.description.as_deref().unwrap_or("");
                if cfg_desc != lock_desc {
                    changes.push(FieldChange {
                        field: "description".to_string(),
                        old: lock_desc.to_string(),
                        new: cfg_desc.to_string(),
                    });
                }
                if badge_cfg.enabled != lock.enabled {
                    changes.push(FieldChange {
                        field: "enabled".to_string(),
                        old: lock.enabled.to_string(),
                        new: badge_cfg.enabled.to_string(),
                    });
                }

                // Badge icon is tracked separately
                if let Some(icon) = &badge_cfg.icon {
                    let full_path = config_dir.join(icon);
                    let current_hash = hash_file(&full_path)?;
                    let lock_hash = lock.icon_hash.as_deref().unwrap_or("");
                    if current_hash != lock_hash {
                        changes.push(FieldChange {
                            field: "icon".to_string(),
                            old: lock_hash.chars().take(8).collect::<String>() + "...",
                            new: current_hash.chars().take(8).collect::<String>() + "...",
                        });
                    }
                }

                if changes.is_empty() {
                    actions.push(ResourceAction {
                        name: name.clone(),
                        action: Action::Skip,
                    });
                } else {
                    actions.push(ResourceAction {
                        name: name.clone(),
                        action: Action::Update { changes },
                    });
                }
            }
        }
    }

    Ok(actions)
}

fn diff_products(
    config: &Config,
    lockfile: &Lockfile,
    config_dir: &Path,
) -> Result<Vec<ResourceAction>> {
    let mut actions = Vec::new();

    for (name, product_cfg) in &config.products {
        match lockfile.products.get(name) {
            None => {
                actions.push(ResourceAction {
                    name: name.clone(),
                    action: Action::Create,
                });
            }
            Some(lock) => {
                let mut changes = Vec::new();

                let cfg_name = resolve_name(product_cfg.name.as_deref(), name);
                if cfg_name != lock.name {
                    changes.push(FieldChange {
                        field: "name".to_string(),
                        old: lock.name.clone(),
                        new: cfg_name.to_string(),
                    });
                }
                if product_cfg.price != lock.price {
                    changes.push(FieldChange {
                        field: "price".to_string(),
                        old: lock.price.to_string(),
                        new: product_cfg.price.to_string(),
                    });
                }
                let cfg_desc = product_cfg.description.as_deref().unwrap_or("");
                let lock_desc = lock.description.as_deref().unwrap_or("");
                if cfg_desc != lock_desc {
                    changes.push(FieldChange {
                        field: "description".to_string(),
                        old: lock_desc.to_string(),
                        new: cfg_desc.to_string(),
                    });
                }
                if product_cfg.for_sale != lock.for_sale {
                    changes.push(FieldChange {
                        field: "for_sale".to_string(),
                        old: lock.for_sale.to_string(),
                        new: product_cfg.for_sale.to_string(),
                    });
                }
                if product_cfg.regional_pricing != lock.regional_pricing {
                    changes.push(FieldChange {
                        field: "regional_pricing".to_string(),
                        old: lock.regional_pricing.to_string(),
                        new: product_cfg.regional_pricing.to_string(),
                    });
                }
                if product_cfg.store_page != lock.store_page {
                    changes.push(FieldChange {
                        field: "store_page".to_string(),
                        old: lock.store_page.to_string(),
                        new: product_cfg.store_page.to_string(),
                    });
                }

                if let Some(icon) = &product_cfg.icon {
                    let full_path = config_dir.join(icon);
                    let current_hash = hash_file(&full_path)?;
                    let lock_hash = lock.icon_hash.as_deref().unwrap_or("");
                    if current_hash != lock_hash {
                        changes.push(FieldChange {
                            field: "icon".to_string(),
                            old: lock_hash.chars().take(8).collect::<String>() + "...",
                            new: current_hash.chars().take(8).collect::<String>() + "...",
                        });
                    }
                }

                if changes.is_empty() {
                    actions.push(ResourceAction {
                        name: name.clone(),
                        action: Action::Skip,
                    });
                } else {
                    actions.push(ResourceAction {
                        name: name.clone(),
                        action: Action::Update { changes },
                    });
                }
            }
        }
    }

    Ok(actions)
}
