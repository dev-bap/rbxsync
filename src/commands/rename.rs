use std::collections::BTreeMap;

use anyhow::{bail, Result};

use crate::cli::{Cli, ResourceType};
use crate::config::Config;
use crate::lockfile::{Lockfile, LOCKFILE_NAME};

pub fn run(cli: &Cli, resource: ResourceType, old_key: &str, new_key: &str) -> Result<()> {
    let config_path = &cli.config;
    let lockfile_path = config_path
        .parent()
        .unwrap_or(std::path::Path::new("."))
        .join(LOCKFILE_NAME);

    let mut config = Config::load(config_path)?;
    let mut lockfile = Lockfile::load(&lockfile_path)?;

    let type_label = match resource {
        ResourceType::Passes => "pass",
        ResourceType::Badges => "badge",
        ResourceType::Products => "product",
    };

    rename_in_maps(
        &resource,
        &mut config,
        &mut lockfile,
        old_key,
        new_key,
        type_label,
    )?;

    config.save(config_path)?;
    lockfile.save(&lockfile_path)?;

    println!("Renamed {type_label} '{old_key}' -> '{new_key}'");
    Ok(())
}

fn rename_in_maps(
    resource: &ResourceType,
    config: &mut Config,
    lockfile: &mut Lockfile,
    old_key: &str,
    new_key: &str,
    type_label: &str,
) -> Result<()> {
    match resource {
        ResourceType::Passes => {
            rename_entry(&mut config.passes, old_key, new_key, type_label)?;
            if config.passes[new_key].name.is_none() {
                config.passes.get_mut(new_key).unwrap().name = Some(old_key.to_string());
            }
            rename_entry(&mut lockfile.passes, old_key, new_key, type_label).ok();
        }
        ResourceType::Badges => {
            rename_entry(&mut config.badges, old_key, new_key, type_label)?;
            if config.badges[new_key].name.is_none() {
                config.badges.get_mut(new_key).unwrap().name = Some(old_key.to_string());
            }
            rename_entry(&mut lockfile.badges, old_key, new_key, type_label).ok();
        }
        ResourceType::Products => {
            rename_entry(&mut config.products, old_key, new_key, type_label)?;
            if config.products[new_key].name.is_none() {
                config.products.get_mut(new_key).unwrap().name = Some(old_key.to_string());
            }
            rename_entry(&mut lockfile.products, old_key, new_key, type_label).ok();
        }
    }
    Ok(())
}

fn rename_entry<V>(
    map: &mut BTreeMap<String, V>,
    old_key: &str,
    new_key: &str,
    type_label: &str,
) -> Result<()> {
    if !map.contains_key(old_key) {
        bail!("{type_label} '{old_key}' not found");
    }
    if map.contains_key(new_key) {
        bail!("{type_label} '{new_key}' already exists");
    }
    let value = map.remove(old_key).unwrap();
    map.insert(new_key.to_string(), value);
    Ok(())
}
