use std::collections::BTreeMap;

use rbxsync::lockfile::{BadgeLock, Lockfile, PassLock, ProductLock};

#[test]
fn round_trip() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("test.lock.toml");

    let original = Lockfile {
        version: 1,
        universe_id: 12345,
        passes: BTreeMap::from([(
            "VIP".into(),
            PassLock {
                id: 111,
                name: "VIP".into(),
                price: Some(499),
                description: Some("VIP access".into()),
                icon_asset_id: Some(999),
                icon_hash: Some("abc123".into()),
                for_sale: true,
                regional_pricing: false,
            },
        )]),
        badges: BTreeMap::from([(
            "Welcome".into(),
            BadgeLock {
                id: 222,
                name: "Welcome".into(),
                description: Some("Welcome!".into()),
                enabled: true,
                icon_asset_id: None,
                icon_hash: None,
            },
        )]),
        products: BTreeMap::from([(
            "Coins".into(),
            ProductLock {
                id: 333,
                name: "Coins".into(),
                price: 99,
                description: None,
                icon_asset_id: None,
                icon_hash: None,
                for_sale: true,
                regional_pricing: false,
                store_page: true,
            },
        )]),
    };

    original.save(&path).unwrap();
    let loaded = Lockfile::load(&path).unwrap();

    assert_eq!(loaded, original);
}

#[test]
fn load_nonexistent_returns_default() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("does_not_exist.toml");

    let lockfile = Lockfile::load(&path).unwrap();
    assert_eq!(lockfile.version, 0);
    assert_eq!(lockfile.universe_id, 0);
    assert!(lockfile.passes.is_empty());
    assert!(lockfile.badges.is_empty());
    assert!(lockfile.products.is_empty());
}

#[test]
fn load_with_extra_fields() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("test.toml");

    std::fs::write(
        &path,
        r#"
version = 1
universe_id = 100
some_unknown_field = "hello"

[passes.VIP]
id = 1
name = "VIP"
extra_field = true
"#,
    )
    .unwrap();

    let lockfile = Lockfile::load(&path).unwrap();
    assert_eq!(lockfile.universe_id, 100);
    assert_eq!(lockfile.passes["VIP"].id, 1);
}

#[test]
fn save_creates_valid_toml() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("test.toml");

    let lockfile = Lockfile {
        version: 1,
        universe_id: 42,
        passes: BTreeMap::new(),
        badges: BTreeMap::new(),
        products: BTreeMap::new(),
    };

    lockfile.save(&path).unwrap();
    assert!(path.exists());

    let content = std::fs::read_to_string(&path).unwrap();
    let parsed: toml::Value = toml::from_str(&content).unwrap();
    assert_eq!(parsed["version"].as_integer(), Some(1));
    assert_eq!(parsed["universe_id"].as_integer(), Some(42));
}
