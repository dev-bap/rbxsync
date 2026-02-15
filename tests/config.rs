use std::collections::BTreeMap;

use rbxsync::config::Config;

#[test]
fn parse_minimal_config() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("rbxsync.toml");
    std::fs::write(
        &path,
        r#"
[experience]
universe_id = 12345

[experience.creator]
type = "user"
id = 67890
"#,
    )
    .unwrap();

    let config = Config::load(&path).unwrap();
    assert_eq!(config.experience.universe_id, 12345);
    assert_eq!(config.experience.creator.id, 67890);
    assert!(config.passes.is_empty());
    assert!(config.badges.is_empty());
    assert!(config.products.is_empty());
}

#[test]
fn parse_full_config() {
    let dir = tempfile::tempdir().unwrap();
    let config_dir = dir.path();

    // Create icon files so validation passes
    std::fs::write(config_dir.join("vip.png"), b"fake").unwrap();
    std::fs::write(config_dir.join("welcome.png"), b"fake").unwrap();
    std::fs::write(config_dir.join("coins.png"), b"fake").unwrap();

    let path = config_dir.join("rbxsync.toml");
    std::fs::write(
        &path,
        r#"
[experience]
universe_id = 12345

[experience.creator]
type = "group"
id = 999

[codegen]
output = "src/GameIds.luau"

[icons]
bleed = false
dir = "my_icons"

[passes.VIP]
price = 499
description = "VIP access"
icon = "vip.png"

[badges.Welcome]
description = "Welcome!"
icon = "welcome.png"
enabled = false

[products.Coins100]
price = 99
description = "100 coins"
icon = "coins.png"
"#,
    )
    .unwrap();

    let config = Config::load(&path).unwrap();
    assert_eq!(config.experience.universe_id, 12345);
    assert_eq!(config.experience.creator.id, 999);
    assert_eq!(
        config.codegen.output.as_deref().unwrap().to_str().unwrap(),
        "src/GameIds.luau"
    );
    assert!(!config.icons.bleed);
    assert_eq!(config.icons.dir.to_str().unwrap(), "my_icons");
    assert_eq!(config.passes.len(), 1);
    assert_eq!(config.passes["VIP"].price, Some(499));
    assert_eq!(config.badges.len(), 1);
    assert!(!config.badges["Welcome"].enabled);
    assert_eq!(config.products.len(), 1);
    assert_eq!(config.products["Coins100"].price, 99);
}

#[test]
fn missing_experience_section() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("rbxsync.toml");
    std::fs::write(&path, "").unwrap();

    assert!(Config::load(&path).is_err());
}

#[test]
fn missing_creator() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("rbxsync.toml");
    std::fs::write(
        &path,
        r#"
[experience]
universe_id = 12345
"#,
    )
    .unwrap();

    assert!(Config::load(&path).is_err());
}

#[test]
fn invalid_creator_type() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("rbxsync.toml");
    std::fs::write(
        &path,
        r#"
[experience]
universe_id = 12345

[experience.creator]
type = "invalid"
id = 1
"#,
    )
    .unwrap();

    assert!(Config::load(&path).is_err());
}

#[test]
fn default_template_is_valid_toml() {
    let template = Config::default_template();
    let result: Result<toml::Value, _> = toml::from_str(&template);
    assert!(
        result.is_ok(),
        "default template is not valid TOML: {:?}",
        result.err()
    );
}

#[test]
fn skip_serializing_empty_maps() {
    let config = Config {
        experience: rbxsync::config::Experience {
            universe_id: 1,
            creator: rbxsync::config::Creator {
                creator_type: rbxsync::config::CreatorType::User,
                id: 1,
            },
        },
        codegen: Default::default(),
        icons: Default::default(),
        passes: BTreeMap::new(),
        badges: BTreeMap::new(),
        products: BTreeMap::new(),
    };

    let serialized = toml::to_string(&config).unwrap();
    assert!(!serialized.contains("[passes"));
    assert!(!serialized.contains("[badges"));
    assert!(!serialized.contains("[products"));
}

#[test]
fn icon_validation_missing_file() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("rbxsync.toml");
    std::fs::write(
        &path,
        r#"
[experience]
universe_id = 1

[experience.creator]
type = "user"
id = 1

[passes.VIP]
icon = "nonexistent.png"
"#,
    )
    .unwrap();

    let err = Config::load(&path).unwrap_err();
    assert!(err.to_string().contains("does not exist"), "error: {}", err);
}

#[test]
fn icon_validation_existing_file() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("icon.png"), b"fake").unwrap();

    let path = dir.path().join("rbxsync.toml");
    std::fs::write(
        &path,
        r#"
[experience]
universe_id = 1

[experience.creator]
type = "user"
id = 1

[passes.VIP]
icon = "icon.png"
"#,
    )
    .unwrap();

    assert!(Config::load(&path).is_ok());
}

#[test]
fn default_values() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("rbxsync.toml");
    std::fs::write(
        &path,
        r#"
[experience]
universe_id = 1

[experience.creator]
type = "user"
id = 1

[passes.VIP]
price = 100

[badges.Welcome]
description = "Hello"

[products.Coins]
price = 50
"#,
    )
    .unwrap();

    let config = Config::load(&path).unwrap();
    // Default icon settings
    assert!(config.icons.bleed);
    assert_eq!(config.icons.dir.to_str().unwrap(), "icons");
    // Badge enabled defaults to true
    assert!(config.badges["Welcome"].enabled);
    // Badge name defaults to None
    assert!(config.badges["Welcome"].name.is_none());
    // Pass defaults
    assert!(config.passes["VIP"].name.is_none());
    assert!(config.passes["VIP"].for_sale);
    assert!(!config.passes["VIP"].regional_pricing);
    // Product defaults
    assert!(config.products["Coins"].name.is_none());
    assert!(config.products["Coins"].for_sale);
    assert!(!config.products["Coins"].regional_pricing);
    assert!(!config.products["Coins"].store_page);
}

#[test]
fn new_fields_parsed() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("rbxsync.toml");
    std::fs::write(
        &path,
        r#"
[experience]
universe_id = 1

[experience.creator]
type = "user"
id = 1

[passes.vip]
name = "VIP Pass"
price = 499
for_sale = false
regional_pricing = true

[badges.welcome]
name = "Welcome Badge"

[products.coins]
name = "100 Coins"
price = 99
for_sale = false
regional_pricing = true
store_page = true
"#,
    )
    .unwrap();

    let config = Config::load(&path).unwrap();
    assert_eq!(config.passes["vip"].name.as_deref(), Some("VIP Pass"));
    assert!(!config.passes["vip"].for_sale);
    assert!(config.passes["vip"].regional_pricing);
    assert_eq!(
        config.badges["welcome"].name.as_deref(),
        Some("Welcome Badge")
    );
    assert_eq!(config.products["coins"].name.as_deref(), Some("100 Coins"));
    assert!(!config.products["coins"].for_sale);
    assert!(config.products["coins"].regional_pricing);
    assert!(config.products["coins"].store_page);
}

#[test]
fn resolve_name_helper() {
    use rbxsync::config::resolve_name;

    assert_eq!(resolve_name(Some("VIP Pass"), "vip"), "VIP Pass");
    assert_eq!(resolve_name(None, "vip"), "vip");
}
