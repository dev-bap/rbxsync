use rbxsync::codegen::{
    build_tree, build_tree_default, build_tree_default_flat, format_key, generate_luau,
    generate_typescript, is_valid_luau_identifier,
};
use rbxsync::config::{
    BadgeConfig, CodegenConfig, CodegenPaths, CodegenStyle, Config, Creator, CreatorType,
    Experience, IconsConfig, PassConfig, ProductConfig,
};
use rbxsync::lockfile::{BadgeLock, Lockfile, PassLock, ProductLock};
use std::collections::BTreeMap;

#[test]
fn generate_luau_with_all_sections() {
    let dir = tempfile::tempdir().unwrap();
    let output = dir.path().join("GameIds.luau");

    let mut lockfile = Lockfile::default();
    lockfile.passes.insert(
        "VIP".into(),
        PassLock {
            id: 111,
            name: "VIP".into(),
            price: Some(499),
            description: None,
            icon_asset_id: None,
            icon_hash: None,
            for_sale: true,
            regional_pricing: false,
        },
    );
    lockfile.badges.insert(
        "Welcome".into(),
        BadgeLock {
            id: 222,
            name: "Welcome".into(),
            description: None,
            enabled: true,
            icon_asset_id: None,
            icon_hash: None,
        },
    );
    lockfile.products.insert(
        "Coins100".into(),
        ProductLock {
            id: 333,
            name: "Coins100".into(),
            price: 99,
            description: None,
            icon_asset_id: None,
            icon_hash: None,
            for_sale: true,
            regional_pricing: false,
            store_page: false,
        },
    );

    let tree = build_tree_default(&lockfile);
    generate_luau(&tree, &output).unwrap();
    let content = std::fs::read_to_string(&output).unwrap();

    assert!(content.contains("local GameIds = {"));
    assert!(content.contains("return GameIds"));
    assert!(content.contains("passes = {"));
    assert!(content.contains("VIP = 111,"));
    assert!(content.contains("badges = {"));
    assert!(content.contains("Welcome = 222,"));
    assert!(content.contains("products = {"));
    assert!(content.contains("Coins100 = 333,"));
}

#[test]
fn generate_luau_empty_lockfile() {
    let dir = tempfile::tempdir().unwrap();
    let output = dir.path().join("GameIds.luau");

    let lockfile = Lockfile::default();
    let tree = build_tree_default(&lockfile);
    generate_luau(&tree, &output).unwrap();
    let content = std::fs::read_to_string(&output).unwrap();

    assert!(content.contains("local GameIds = {"));
    assert!(content.contains("return GameIds"));
    assert!(!content.contains("passes"));
    assert!(!content.contains("badges"));
    assert!(!content.contains("products"));
}

#[test]
fn generate_luau_escaped_keys() {
    let dir = tempfile::tempdir().unwrap();
    let output = dir.path().join("GameIds.luau");

    let mut lockfile = Lockfile::default();
    lockfile.passes.insert(
        "my-pass".into(),
        PassLock {
            id: 1,
            name: "my-pass".into(),
            price: None,
            description: None,
            icon_asset_id: None,
            icon_hash: None,
            for_sale: true,
            regional_pricing: false,
        },
    );
    lockfile.passes.insert(
        "100coins".into(),
        PassLock {
            id: 2,
            name: "100coins".into(),
            price: None,
            description: None,
            icon_asset_id: None,
            icon_hash: None,
            for_sale: true,
            regional_pricing: false,
        },
    );
    lockfile.passes.insert(
        "type".into(),
        PassLock {
            id: 3,
            name: "type".into(),
            price: None,
            description: None,
            icon_asset_id: None,
            icon_hash: None,
            for_sale: true,
            regional_pricing: false,
        },
    );

    let tree = build_tree_default(&lockfile);
    generate_luau(&tree, &output).unwrap();
    let content = std::fs::read_to_string(&output).unwrap();

    assert!(content.contains(r#"["my-pass"] = 1,"#));
    assert!(content.contains(r#"["100coins"] = 2,"#));
    assert!(content.contains(r#"["type"] = 3,"#));
}

#[test]
fn generate_luau_custom_filename() {
    let dir = tempfile::tempdir().unwrap();
    let output = dir.path().join("GameAssets.luau");

    let lockfile = Lockfile::default();
    let tree = build_tree_default(&lockfile);
    generate_luau(&tree, &output).unwrap();
    let content = std::fs::read_to_string(&output).unwrap();

    assert!(content.contains("local GameAssets = {"));
    assert!(content.contains("return GameAssets"));
}

#[test]
fn valid_luau_identifiers() {
    assert!(is_valid_luau_identifier("VIP"));
    assert!(is_valid_luau_identifier("_test"));
    assert!(is_valid_luau_identifier("abc123"));
    assert!(is_valid_luau_identifier("hello_world"));
}

#[test]
fn invalid_luau_identifiers() {
    assert!(!is_valid_luau_identifier(""));
    assert!(!is_valid_luau_identifier("my-pass"));
    assert!(!is_valid_luau_identifier("123abc"));
    assert!(!is_valid_luau_identifier("type"));
    assert!(!is_valid_luau_identifier("end"));
}

#[test]
fn format_key_bare_vs_escaped() {
    assert_eq!(format_key("VIP"), "VIP");
    assert_eq!(format_key("_test"), "_test");
    assert_eq!(format_key("my-pass"), r#"["my-pass"]"#);
    assert_eq!(format_key("123abc"), r#"["123abc"]"#);
    assert_eq!(format_key("type"), r#"["type"]"#);
}

// ---------------------------------------------------------------------------
// Helper to build a minimal Config for codegen path tests
// ---------------------------------------------------------------------------

fn test_config_full(
    style: CodegenStyle,
    codegen_paths: CodegenPaths,
    extra: BTreeMap<String, u64>,
    passes: BTreeMap<String, PassConfig>,
    badges: BTreeMap<String, BadgeConfig>,
    products: BTreeMap<String, ProductConfig>,
) -> Config {
    Config {
        experience: Experience {
            universe_id: 1,
            creator: Creator {
                creator_type: CreatorType::User,
                id: 1,
            },
        },
        codegen: CodegenConfig {
            output: None,
            typescript: false,
            style,
            paths: codegen_paths,
            extra,
        },
        icons: IconsConfig::default(),
        passes,
        badges,
        products,
    }
}

fn test_config_with_style(
    style: CodegenStyle,
    codegen_paths: CodegenPaths,
    passes: BTreeMap<String, PassConfig>,
    badges: BTreeMap<String, BadgeConfig>,
    products: BTreeMap<String, ProductConfig>,
) -> Config {
    test_config_full(
        style,
        codegen_paths,
        BTreeMap::new(),
        passes,
        badges,
        products,
    )
}

fn test_config(
    codegen_paths: CodegenPaths,
    passes: BTreeMap<String, PassConfig>,
    badges: BTreeMap<String, BadgeConfig>,
    products: BTreeMap<String, ProductConfig>,
) -> Config {
    test_config_with_style(
        CodegenStyle::Nested,
        codegen_paths,
        passes,
        badges,
        products,
    )
}

// ---------------------------------------------------------------------------
// New path tests
// ---------------------------------------------------------------------------

#[test]
fn generate_luau_with_section_paths() {
    let dir = tempfile::tempdir().unwrap();
    let output = dir.path().join("GameIds.luau");

    let mut lockfile = Lockfile::default();
    lockfile.passes.insert(
        "vip_1".into(),
        PassLock {
            id: 100,
            name: "vip_1".into(),
            price: Some(599),
            description: None,
            icon_asset_id: None,
            icon_hash: None,
            for_sale: true,
            regional_pricing: false,
        },
    );
    lockfile.products.insert(
        "coins_100".into(),
        ProductLock {
            id: 200,
            name: "coins_100".into(),
            price: 99,
            description: None,
            icon_asset_id: None,
            icon_hash: None,
            for_sale: true,
            regional_pricing: false,
            store_page: false,
        },
    );

    let config = test_config(
        CodegenPaths {
            passes: Some("player.vips".into()),
            badges: None,
            products: Some("shop.items".into()),
        },
        BTreeMap::from([(
            "vip_1".into(),
            PassConfig {
                name: None,
                price: Some(599),
                description: None,
                icon: None,
                for_sale: true,
                regional_pricing: false,
                path: None,
            },
        )]),
        BTreeMap::new(),
        BTreeMap::from([(
            "coins_100".into(),
            ProductConfig {
                name: None,
                price: 99,
                description: None,
                icon: None,
                for_sale: true,
                regional_pricing: false,
                store_page: false,
                path: None,
            },
        )]),
    );

    let tree = build_tree(&lockfile, &config);
    generate_luau(&tree, &output).unwrap();
    let content = std::fs::read_to_string(&output).unwrap();

    assert!(content.contains("player = {"));
    assert!(content.contains("vips = {"));
    assert!(content.contains("vip_1 = 100,"));
    assert!(content.contains("shop = {"));
    assert!(content.contains("items = {"));
    assert!(content.contains("coins_100 = 200,"));
    // Should NOT contain flat "passes" or "products" keys
    assert!(!content.contains("passes = {"));
    assert!(!content.contains("products = {"));
}

#[test]
fn generate_luau_with_item_path_override() {
    let dir = tempfile::tempdir().unwrap();
    let output = dir.path().join("GameIds.luau");

    let mut lockfile = Lockfile::default();
    lockfile.products.insert(
        "special_offer".into(),
        ProductLock {
            id: 300,
            name: "special_offer".into(),
            price: 99,
            description: None,
            icon_asset_id: None,
            icon_hash: None,
            for_sale: true,
            regional_pricing: false,
            store_page: false,
        },
    );
    lockfile.products.insert(
        "coins_100".into(),
        ProductLock {
            id: 400,
            name: "coins_100".into(),
            price: 99,
            description: None,
            icon_asset_id: None,
            icon_hash: None,
            for_sale: true,
            regional_pricing: false,
            store_page: false,
        },
    );

    let config = test_config(
        CodegenPaths {
            passes: None,
            badges: None,
            products: Some("shop.items".into()),
        },
        BTreeMap::new(),
        BTreeMap::new(),
        BTreeMap::from([
            (
                "special_offer".into(),
                ProductConfig {
                    name: None,
                    price: 99,
                    description: None,
                    icon: None,
                    for_sale: true,
                    regional_pricing: false,
                    store_page: false,
                    path: Some("shop.specials".into()),
                },
            ),
            (
                "coins_100".into(),
                ProductConfig {
                    name: None,
                    price: 99,
                    description: None,
                    icon: None,
                    for_sale: true,
                    regional_pricing: false,
                    store_page: false,
                    path: None,
                },
            ),
        ]),
    );

    let tree = build_tree(&lockfile, &config);
    generate_luau(&tree, &output).unwrap();
    let content = std::fs::read_to_string(&output).unwrap();

    // special_offer goes to shop.specials (per-item override)
    assert!(content.contains("specials = {"));
    assert!(content.contains("special_offer = 300,"));
    // coins_100 goes to shop.items (section default)
    assert!(content.contains("items = {"));
    assert!(content.contains("coins_100 = 400,"));
    // Both should be under "shop"
    assert!(content.contains("shop = {"));
}

#[test]
fn generate_luau_nested_path_merging() {
    let dir = tempfile::tempdir().unwrap();
    let output = dir.path().join("GameIds.luau");

    let mut lockfile = Lockfile::default();
    lockfile.passes.insert(
        "vip_pass".into(),
        PassLock {
            id: 500,
            name: "vip_pass".into(),
            price: Some(599),
            description: None,
            icon_asset_id: None,
            icon_hash: None,
            for_sale: true,
            regional_pricing: false,
        },
    );
    lockfile.products.insert(
        "coins".into(),
        ProductLock {
            id: 600,
            name: "coins".into(),
            price: 99,
            description: None,
            icon_asset_id: None,
            icon_hash: None,
            for_sale: true,
            regional_pricing: false,
            store_page: false,
        },
    );

    // Both passes and products map under "shop"
    let config = test_config(
        CodegenPaths {
            passes: Some("shop.vips".into()),
            badges: None,
            products: Some("shop.items".into()),
        },
        BTreeMap::from([(
            "vip_pass".into(),
            PassConfig {
                name: None,
                price: Some(599),
                description: None,
                icon: None,
                for_sale: true,
                regional_pricing: false,
                path: None,
            },
        )]),
        BTreeMap::new(),
        BTreeMap::from([(
            "coins".into(),
            ProductConfig {
                name: None,
                price: 99,
                description: None,
                icon: None,
                for_sale: true,
                regional_pricing: false,
                store_page: false,
                path: None,
            },
        )]),
    );

    let tree = build_tree(&lockfile, &config);
    generate_luau(&tree, &output).unwrap();
    let content = std::fs::read_to_string(&output).unwrap();

    // Both should be under the same "shop" parent
    assert!(content.contains("shop = {"));
    assert!(content.contains("vips = {"));
    assert!(content.contains("vip_pass = 500,"));
    assert!(content.contains("items = {"));
    assert!(content.contains("coins = 600,"));

    // "shop" should appear only once
    assert_eq!(content.matches("shop = {").count(), 1);
}

// ---------------------------------------------------------------------------
// Flat style tests
// ---------------------------------------------------------------------------

#[test]
fn generate_luau_flat_default() {
    let dir = tempfile::tempdir().unwrap();
    let output = dir.path().join("GameIds.luau");

    let mut lockfile = Lockfile::default();
    lockfile.passes.insert(
        "VIP".into(),
        PassLock {
            id: 111,
            name: "VIP".into(),
            price: Some(499),
            description: None,
            icon_asset_id: None,
            icon_hash: None,
            for_sale: true,
            regional_pricing: false,
        },
    );
    lockfile.products.insert(
        "Coins100".into(),
        ProductLock {
            id: 333,
            name: "Coins100".into(),
            price: 99,
            description: None,
            icon_asset_id: None,
            icon_hash: None,
            for_sale: true,
            regional_pricing: false,
            store_page: false,
        },
    );

    let tree = build_tree_default_flat(&lockfile);
    generate_luau(&tree, &output).unwrap();
    let content = std::fs::read_to_string(&output).unwrap();

    assert!(content.contains(r#"["passes.VIP"] = 111,"#));
    assert!(content.contains(r#"["products.Coins100"] = 333,"#));
    // Should NOT have nested tables
    assert!(!content.contains("passes = {"));
    assert!(!content.contains("products = {"));
}

#[test]
fn generate_typescript_flat_default() {
    let dir = tempfile::tempdir().unwrap();
    let output = dir.path().join("GameIds.d.ts");

    let mut lockfile = Lockfile::default();
    lockfile.passes.insert(
        "VIP".into(),
        PassLock {
            id: 111,
            name: "VIP".into(),
            price: Some(499),
            description: None,
            icon_asset_id: None,
            icon_hash: None,
            for_sale: true,
            regional_pricing: false,
        },
    );

    let tree = build_tree_default_flat(&lockfile);
    generate_typescript(&tree, &output).unwrap();
    let content = std::fs::read_to_string(&output).unwrap();

    assert!(content.contains(r#""passes.VIP": number"#));
}

#[test]
fn generate_luau_flat_with_custom_paths() {
    let dir = tempfile::tempdir().unwrap();
    let output = dir.path().join("GameIds.luau");

    let mut lockfile = Lockfile::default();
    lockfile.passes.insert(
        "vip_1".into(),
        PassLock {
            id: 100,
            name: "vip_1".into(),
            price: Some(599),
            description: None,
            icon_asset_id: None,
            icon_hash: None,
            for_sale: true,
            regional_pricing: false,
        },
    );
    lockfile.products.insert(
        "coins_100".into(),
        ProductLock {
            id: 200,
            name: "coins_100".into(),
            price: 99,
            description: None,
            icon_asset_id: None,
            icon_hash: None,
            for_sale: true,
            regional_pricing: false,
            store_page: false,
        },
    );
    lockfile.products.insert(
        "special_offer".into(),
        ProductLock {
            id: 300,
            name: "special_offer".into(),
            price: 49,
            description: None,
            icon_asset_id: None,
            icon_hash: None,
            for_sale: true,
            regional_pricing: false,
            store_page: false,
        },
    );

    let config = test_config_with_style(
        CodegenStyle::Flat,
        CodegenPaths {
            passes: Some("player.vips".into()),
            badges: None,
            products: Some("shop.items".into()),
        },
        BTreeMap::from([(
            "vip_1".into(),
            PassConfig {
                name: None,
                price: Some(599),
                description: None,
                icon: None,
                for_sale: true,
                regional_pricing: false,
                path: None,
            },
        )]),
        BTreeMap::new(),
        BTreeMap::from([
            (
                "coins_100".into(),
                ProductConfig {
                    name: None,
                    price: 99,
                    description: None,
                    icon: None,
                    for_sale: true,
                    regional_pricing: false,
                    store_page: false,
                    path: None,
                },
            ),
            (
                "special_offer".into(),
                ProductConfig {
                    name: None,
                    price: 49,
                    description: None,
                    icon: None,
                    for_sale: true,
                    regional_pricing: false,
                    store_page: false,
                    path: Some("shop.specials".into()),
                },
            ),
        ]),
    );

    let tree = build_tree(&lockfile, &config);
    generate_luau(&tree, &output).unwrap();
    let content = std::fs::read_to_string(&output).unwrap();

    // Flat keys with dot-separated paths
    assert!(content.contains(r#"["player.vips.vip_1"] = 100,"#));
    assert!(content.contains(r#"["shop.items.coins_100"] = 200,"#));
    assert!(content.contains(r#"["shop.specials.special_offer"] = 300,"#));
    // No nesting
    assert!(!content.contains("player = {"));
    assert!(!content.contains("shop = {"));
}

#[test]
fn flat_style_is_config_default() {
    // Verify that a config with no explicit style uses flat
    let toml_str = r#"
[experience]
universe_id = 1

[experience.creator]
type = "user"
id = 1

[codegen]
output = "GameIds.luau"

[passes.VIP]
price = 499
"#;
    let config: Config = toml::from_str(toml_str).unwrap();
    assert_eq!(config.codegen.style, CodegenStyle::Flat);
}

// ---------------------------------------------------------------------------
// Extra entries tests
// ---------------------------------------------------------------------------

#[test]
fn generate_luau_extra_flat() {
    let dir = tempfile::tempdir().unwrap();
    let output = dir.path().join("GameIds.luau");

    let lockfile = Lockfile::default();

    let config = test_config_full(
        CodegenStyle::Flat,
        CodegenPaths::default(),
        BTreeMap::from([
            ("passes.legacy_vip".into(), 111),
            ("passes.old_premium".into(), 222),
            ("products.starter_pack".into(), 333),
        ]),
        BTreeMap::new(),
        BTreeMap::new(),
        BTreeMap::new(),
    );

    let tree = build_tree(&lockfile, &config);
    generate_luau(&tree, &output).unwrap();
    let content = std::fs::read_to_string(&output).unwrap();

    assert!(content.contains(r#"["passes.legacy_vip"] = 111,"#));
    assert!(content.contains(r#"["passes.old_premium"] = 222,"#));
    assert!(content.contains(r#"["products.starter_pack"] = 333,"#));
}

#[test]
fn generate_luau_extra_nested() {
    let dir = tempfile::tempdir().unwrap();
    let output = dir.path().join("GameIds.luau");

    let lockfile = Lockfile::default();

    let config = test_config_full(
        CodegenStyle::Nested,
        CodegenPaths::default(),
        BTreeMap::from([
            ("passes.legacy_vip".into(), 111),
            ("passes.old_premium".into(), 222),
            ("products.starter_pack".into(), 333),
        ]),
        BTreeMap::new(),
        BTreeMap::new(),
        BTreeMap::new(),
    );

    let tree = build_tree(&lockfile, &config);
    generate_luau(&tree, &output).unwrap();
    let content = std::fs::read_to_string(&output).unwrap();

    assert!(content.contains("passes = {"));
    assert!(content.contains("legacy_vip = 111,"));
    assert!(content.contains("old_premium = 222,"));
    assert!(content.contains("products = {"));
    assert!(content.contains("starter_pack = 333,"));
}

#[test]
fn generate_luau_extra_mixed_with_synced() {
    let dir = tempfile::tempdir().unwrap();
    let output = dir.path().join("GameIds.luau");

    let mut lockfile = Lockfile::default();
    lockfile.passes.insert(
        "VIP".into(),
        PassLock {
            id: 100,
            name: "VIP".into(),
            price: Some(499),
            description: None,
            icon_asset_id: None,
            icon_hash: None,
            for_sale: true,
            regional_pricing: false,
        },
    );

    let config = test_config_full(
        CodegenStyle::Nested,
        CodegenPaths::default(),
        BTreeMap::from([("passes.legacy_vip".into(), 999)]),
        BTreeMap::from([(
            "VIP".into(),
            PassConfig {
                name: None,
                price: Some(499),
                description: None,
                icon: None,
                for_sale: true,
                regional_pricing: false,
                path: None,
            },
        )]),
        BTreeMap::new(),
        BTreeMap::new(),
    );

    let tree = build_tree(&lockfile, &config);
    generate_luau(&tree, &output).unwrap();
    let content = std::fs::read_to_string(&output).unwrap();

    // Synced pass
    assert!(content.contains("passes = {"));
    assert!(content.contains("VIP = 100,"));
    // Extra entry merged into same "passes" branch
    assert!(content.contains("legacy_vip = 999,"));
}

#[test]
fn extra_no_dot_inserts_at_root() {
    let dir = tempfile::tempdir().unwrap();
    let output = dir.path().join("GameIds.luau");

    let lockfile = Lockfile::default();

    let config = test_config_full(
        CodegenStyle::Nested,
        CodegenPaths::default(),
        BTreeMap::from([("legacy_vip".into(), 42)]),
        BTreeMap::new(),
        BTreeMap::new(),
        BTreeMap::new(),
    );

    let tree = build_tree(&lockfile, &config);
    generate_luau(&tree, &output).unwrap();
    let content = std::fs::read_to_string(&output).unwrap();

    assert!(content.contains("legacy_vip = 42,"));
}

#[test]
fn extra_parses_from_toml() {
    let toml_str = r#"
[experience]
universe_id = 1

[experience.creator]
type = "user"
id = 1

[codegen]
output = "GameIds.luau"

[codegen.extra]
"passes.legacy_vip" = 1234567
"products.starter_pack" = 9876543
"#;
    let config: Config = toml::from_str(toml_str).unwrap();
    assert_eq!(config.codegen.extra.len(), 2);
    assert_eq!(config.codegen.extra["passes.legacy_vip"], 1234567);
    assert_eq!(config.codegen.extra["products.starter_pack"], 9876543);
}
