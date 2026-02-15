use rbxsync::cli::{Cli, ResourceType};
use rbxsync::config::Config;
use rbxsync::lockfile::{BadgeLock, Lockfile, PassLock, ProductLock, LOCKFILE_NAME};
use std::path::PathBuf;

fn write_config(dir: &std::path::Path, content: &str) -> PathBuf {
    let path = dir.join("rbxsync.toml");
    std::fs::write(&path, content).unwrap();
    path
}

fn write_lockfile(dir: &std::path::Path, lockfile: &Lockfile) {
    let path = dir.join(LOCKFILE_NAME);
    lockfile.save(&path).unwrap();
}

fn make_cli(config_path: PathBuf) -> Cli {
    Cli {
        command: rbxsync::cli::Commands::Check, // unused by rename
        config: config_path,
        api_key: None,
    }
}

fn base_config() -> &'static str {
    r#"
[experience]
universe_id = 1

[experience.creator]
type = "user"
id = 1
"#
}

fn base_lockfile() -> Lockfile {
    Lockfile {
        version: 1,
        universe_id: 1,
        ..Default::default()
    }
}

#[test]
fn basic_rename_pass() {
    let dir = tempfile::tempdir().unwrap();
    let config_path = write_config(
        dir.path(),
        &format!(
            r#"{}
[passes.VIP]
price = 499
"#,
            base_config()
        ),
    );

    let mut lockfile = base_lockfile();
    lockfile.passes.insert(
        "VIP".to_string(),
        PassLock {
            id: 42,
            name: "VIP".to_string(),
            price: Some(499),
            description: None,
            icon_asset_id: None,
            icon_hash: None,
            for_sale: true,
            regional_pricing: false,
        },
    );
    write_lockfile(dir.path(), &lockfile);

    let cli = make_cli(config_path.clone());
    rbxsync::commands::rename::run(&cli, ResourceType::Passes, "VIP", "vip_pass").unwrap();

    let config = Config::load(&config_path).unwrap();
    assert!(!config.passes.contains_key("VIP"));
    assert!(config.passes.contains_key("vip_pass"));
    assert_eq!(config.passes["vip_pass"].price, Some(499));

    let lock = Lockfile::load(&dir.path().join(LOCKFILE_NAME)).unwrap();
    assert!(!lock.passes.contains_key("VIP"));
    assert!(lock.passes.contains_key("vip_pass"));
    assert_eq!(lock.passes["vip_pass"].id, 42);
}

#[test]
fn preserves_display_name_when_no_explicit_name() {
    let dir = tempfile::tempdir().unwrap();
    let config_path = write_config(
        dir.path(),
        &format!(
            r#"{}
[passes.VIP]
price = 499
"#,
            base_config()
        ),
    );
    write_lockfile(dir.path(), &base_lockfile());

    let cli = make_cli(config_path.clone());
    rbxsync::commands::rename::run(&cli, ResourceType::Passes, "VIP", "vip_pass").unwrap();

    let config = Config::load(&config_path).unwrap();
    assert_eq!(config.passes["vip_pass"].name.as_deref(), Some("VIP"));
}

#[test]
fn keeps_explicit_name_unchanged() {
    let dir = tempfile::tempdir().unwrap();
    let config_path = write_config(
        dir.path(),
        &format!(
            r#"{}
[passes.vip]
name = "VIP Pass"
price = 499
"#,
            base_config()
        ),
    );
    write_lockfile(dir.path(), &base_lockfile());

    let cli = make_cli(config_path.clone());
    rbxsync::commands::rename::run(&cli, ResourceType::Passes, "vip", "vip_pass").unwrap();

    let config = Config::load(&config_path).unwrap();
    assert_eq!(config.passes["vip_pass"].name.as_deref(), Some("VIP Pass"));
}

#[test]
fn error_old_key_missing() {
    let dir = tempfile::tempdir().unwrap();
    let config_path = write_config(dir.path(), base_config());
    write_lockfile(dir.path(), &base_lockfile());

    let cli = make_cli(config_path);
    let result = rbxsync::commands::rename::run(&cli, ResourceType::Passes, "nonexistent", "new");
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("not found"));
}

#[test]
fn error_new_key_already_exists() {
    let dir = tempfile::tempdir().unwrap();
    let config_path = write_config(
        dir.path(),
        &format!(
            r#"{}
[passes.VIP]
price = 499

[passes.existing]
price = 100
"#,
            base_config()
        ),
    );
    write_lockfile(dir.path(), &base_lockfile());

    let cli = make_cli(config_path);
    let result = rbxsync::commands::rename::run(&cli, ResourceType::Passes, "VIP", "existing");
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("already exists"));
}

#[test]
fn rename_badge() {
    let dir = tempfile::tempdir().unwrap();
    let config_path = write_config(
        dir.path(),
        &format!(
            r#"{}
[badges.Welcome]
description = "Welcome!"
"#,
            base_config()
        ),
    );

    let mut lockfile = base_lockfile();
    lockfile.badges.insert(
        "Welcome".to_string(),
        BadgeLock {
            id: 10,
            name: "Welcome".to_string(),
            description: Some("Welcome!".to_string()),
            enabled: true,
            icon_asset_id: None,
            icon_hash: None,
        },
    );
    write_lockfile(dir.path(), &lockfile);

    let cli = make_cli(config_path.clone());
    rbxsync::commands::rename::run(&cli, ResourceType::Badges, "Welcome", "welcome_badge").unwrap();

    let config = Config::load(&config_path).unwrap();
    assert!(!config.badges.contains_key("Welcome"));
    assert!(config.badges.contains_key("welcome_badge"));
    assert_eq!(
        config.badges["welcome_badge"].name.as_deref(),
        Some("Welcome")
    );

    let lock = Lockfile::load(&dir.path().join(LOCKFILE_NAME)).unwrap();
    assert!(!lock.badges.contains_key("Welcome"));
    assert!(lock.badges.contains_key("welcome_badge"));
    assert_eq!(lock.badges["welcome_badge"].id, 10);
}

#[test]
fn rename_product() {
    let dir = tempfile::tempdir().unwrap();
    let config_path = write_config(
        dir.path(),
        &format!(
            r#"{}
[products.Coins100]
price = 99
"#,
            base_config()
        ),
    );

    let mut lockfile = base_lockfile();
    lockfile.products.insert(
        "Coins100".to_string(),
        ProductLock {
            id: 55,
            name: "Coins100".to_string(),
            price: 99,
            description: None,
            icon_asset_id: None,
            icon_hash: None,
            for_sale: true,
            regional_pricing: false,
            store_page: false,
        },
    );
    write_lockfile(dir.path(), &lockfile);

    let cli = make_cli(config_path.clone());
    rbxsync::commands::rename::run(&cli, ResourceType::Products, "Coins100", "coins_100").unwrap();

    let config = Config::load(&config_path).unwrap();
    assert!(!config.products.contains_key("Coins100"));
    assert!(config.products.contains_key("coins_100"));
    assert_eq!(
        config.products["coins_100"].name.as_deref(),
        Some("Coins100")
    );

    let lock = Lockfile::load(&dir.path().join(LOCKFILE_NAME)).unwrap();
    assert!(!lock.products.contains_key("Coins100"));
    assert!(lock.products.contains_key("coins_100"));
    assert_eq!(lock.products["coins_100"].id, 55);
}
