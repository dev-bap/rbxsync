use std::collections::BTreeMap;
use std::path::Path;

use rbxsync::config::{
    BadgeConfig, Config, Creator, CreatorType, Experience, PassConfig, ProductConfig,
};
use rbxsync::diff::{build_sync_plan, Action};
use rbxsync::lockfile::{BadgeLock, Lockfile, PassLock, ProductLock};

fn make_config(
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
        codegen: Default::default(),
        icons: Default::default(),
        passes,
        badges,
        products,
    }
}

// --- Pass tests ---

#[test]
fn new_pass_creates() {
    let config = make_config(
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
    let lockfile = Lockfile::default();

    let plan = build_sync_plan(&config, &lockfile, Path::new(".")).unwrap();
    assert_eq!(plan.passes.len(), 1);
    assert!(matches!(plan.passes[0].action, Action::Create));
}

#[test]
fn same_pass_skips() {
    let config = make_config(
        BTreeMap::from([(
            "VIP".into(),
            PassConfig {
                name: None,
                price: Some(499),
                description: Some("VIP access".into()),
                icon: None,
                for_sale: true,
                regional_pricing: false,
                path: None,
            },
        )]),
        BTreeMap::new(),
        BTreeMap::new(),
    );
    let lockfile = Lockfile {
        passes: BTreeMap::from([(
            "VIP".into(),
            PassLock {
                id: 1,
                name: "VIP".into(),
                price: Some(499),
                description: Some("VIP access".into()),
                icon_asset_id: None,
                icon_hash: None,
                for_sale: true,
                regional_pricing: false,
            },
        )]),
        ..Default::default()
    };

    let plan = build_sync_plan(&config, &lockfile, Path::new(".")).unwrap();
    assert_eq!(plan.passes.len(), 1);
    assert!(matches!(plan.passes[0].action, Action::Skip));
}

#[test]
fn changed_pass_price_updates() {
    let config = make_config(
        BTreeMap::from([(
            "VIP".into(),
            PassConfig {
                name: None,
                price: Some(999),
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
    let lockfile = Lockfile {
        passes: BTreeMap::from([(
            "VIP".into(),
            PassLock {
                id: 1,
                name: "VIP".into(),
                price: Some(499),
                description: None,
                icon_asset_id: None,
                icon_hash: None,
                for_sale: true,
                regional_pricing: false,
            },
        )]),
        ..Default::default()
    };

    let plan = build_sync_plan(&config, &lockfile, Path::new(".")).unwrap();
    assert_eq!(plan.passes.len(), 1);
    match &plan.passes[0].action {
        Action::Update { changes } => {
            assert!(changes.iter().any(|c| c.field == "price"));
        }
        other => panic!("expected Update, got {:?}", other),
    }
}

#[test]
fn changed_pass_description_updates() {
    let config = make_config(
        BTreeMap::from([(
            "VIP".into(),
            PassConfig {
                name: None,
                price: Some(499),
                description: Some("New desc".into()),
                icon: None,
                for_sale: true,
                regional_pricing: false,
                path: None,
            },
        )]),
        BTreeMap::new(),
        BTreeMap::new(),
    );
    let lockfile = Lockfile {
        passes: BTreeMap::from([(
            "VIP".into(),
            PassLock {
                id: 1,
                name: "VIP".into(),
                price: Some(499),
                description: Some("Old desc".into()),
                icon_asset_id: None,
                icon_hash: None,
                for_sale: true,
                regional_pricing: false,
            },
        )]),
        ..Default::default()
    };

    let plan = build_sync_plan(&config, &lockfile, Path::new(".")).unwrap();
    match &plan.passes[0].action {
        Action::Update { changes } => {
            let desc = changes.iter().find(|c| c.field == "description").unwrap();
            assert_eq!(desc.old, "Old desc");
            assert_eq!(desc.new, "New desc");
        }
        other => panic!("expected Update, got {:?}", other),
    }
}

#[test]
fn changed_pass_icon_updates() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("icon.png"), b"new icon content").unwrap();

    let config = make_config(
        BTreeMap::from([(
            "VIP".into(),
            PassConfig {
                name: None,
                price: Some(499),
                description: None,
                icon: Some("icon.png".into()),
                for_sale: true,
                regional_pricing: false,
                path: None,
            },
        )]),
        BTreeMap::new(),
        BTreeMap::new(),
    );
    let lockfile = Lockfile {
        passes: BTreeMap::from([(
            "VIP".into(),
            PassLock {
                id: 1,
                name: "VIP".into(),
                price: Some(499),
                description: None,
                icon_asset_id: Some(100),
                icon_hash: Some("oldhash00000".into()),
                for_sale: true,
                regional_pricing: false,
            },
        )]),
        ..Default::default()
    };

    let plan = build_sync_plan(&config, &lockfile, dir.path()).unwrap();
    match &plan.passes[0].action {
        Action::Update { changes } => {
            assert!(changes.iter().any(|c| c.field == "icon"));
        }
        other => panic!("expected Update, got {:?}", other),
    }
}

#[test]
fn pass_in_lockfile_not_in_config_warns() {
    let config = make_config(BTreeMap::new(), BTreeMap::new(), BTreeMap::new());
    let lockfile = Lockfile {
        passes: BTreeMap::from([(
            "OldPass".into(),
            PassLock {
                id: 1,
                name: "OldPass".into(),
                price: None,
                description: None,
                icon_asset_id: None,
                icon_hash: None,
                for_sale: true,
                regional_pricing: false,
            },
        )]),
        ..Default::default()
    };

    let plan = build_sync_plan(&config, &lockfile, Path::new(".")).unwrap();
    assert!(plan.warnings.iter().any(|w| w.contains("OldPass")));
}

// --- has_changes / summary tests ---

#[test]
fn has_changes_all_skip() {
    let config = make_config(
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
    let lockfile = Lockfile {
        passes: BTreeMap::from([(
            "VIP".into(),
            PassLock {
                id: 1,
                name: "VIP".into(),
                price: Some(499),
                description: None,
                icon_asset_id: None,
                icon_hash: None,
                for_sale: true,
                regional_pricing: false,
            },
        )]),
        ..Default::default()
    };

    let plan = build_sync_plan(&config, &lockfile, Path::new(".")).unwrap();
    assert!(!plan.has_changes());
}

#[test]
fn has_changes_with_create() {
    let config = make_config(
        BTreeMap::from([(
            "New".into(),
            PassConfig {
                name: None,
                price: None,
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
    let lockfile = Lockfile::default();

    let plan = build_sync_plan(&config, &lockfile, Path::new(".")).unwrap();
    assert!(plan.has_changes());
}

#[test]
fn summary_counts() {
    let config = make_config(
        BTreeMap::from([
            (
                "New".into(),
                PassConfig {
                    name: None,
                    price: None,
                    description: None,
                    icon: None,
                    for_sale: true,
                    regional_pricing: false,
                    path: None,
                },
            ),
            (
                "Same".into(),
                PassConfig {
                    name: None,
                    price: Some(100),
                    description: None,
                    icon: None,
                    for_sale: true,
                    regional_pricing: false,
                    path: None,
                },
            ),
            (
                "Changed".into(),
                PassConfig {
                    name: None,
                    price: Some(200),
                    description: None,
                    icon: None,
                    for_sale: true,
                    regional_pricing: false,
                    path: None,
                },
            ),
        ]),
        BTreeMap::new(),
        BTreeMap::new(),
    );
    let lockfile = Lockfile {
        passes: BTreeMap::from([
            (
                "Same".into(),
                PassLock {
                    id: 1,
                    name: "Same".into(),
                    price: Some(100),
                    description: None,
                    icon_asset_id: None,
                    icon_hash: None,
                    for_sale: true,
                    regional_pricing: false,
                },
            ),
            (
                "Changed".into(),
                PassLock {
                    id: 2,
                    name: "Changed".into(),
                    price: Some(50),
                    description: None,
                    icon_asset_id: None,
                    icon_hash: None,
                    for_sale: true,
                    regional_pricing: false,
                },
            ),
        ]),
        ..Default::default()
    };

    let plan = build_sync_plan(&config, &lockfile, Path::new(".")).unwrap();
    let summary = plan.summary();
    assert!(summary.contains("1 to create"));
    assert!(summary.contains("1 to update"));
    assert!(summary.contains("1 unchanged"));
}

// --- Badge tests ---

#[test]
fn new_badge_creates() {
    let config = make_config(
        BTreeMap::new(),
        BTreeMap::from([(
            "Welcome".into(),
            BadgeConfig {
                name: None,
                description: Some("Welcome!".into()),
                icon: None,
                enabled: true,
                path: None,
            },
        )]),
        BTreeMap::new(),
    );
    let lockfile = Lockfile::default();

    let plan = build_sync_plan(&config, &lockfile, Path::new(".")).unwrap();
    assert_eq!(plan.badges.len(), 1);
    assert!(matches!(plan.badges[0].action, Action::Create));
}

#[test]
fn badge_description_change_updates() {
    let config = make_config(
        BTreeMap::new(),
        BTreeMap::from([(
            "Welcome".into(),
            BadgeConfig {
                name: None,
                description: Some("New welcome".into()),
                icon: None,
                enabled: true,
                path: None,
            },
        )]),
        BTreeMap::new(),
    );
    let lockfile = Lockfile {
        badges: BTreeMap::from([(
            "Welcome".into(),
            BadgeLock {
                id: 1,
                name: "Welcome".into(),
                description: Some("Old welcome".into()),
                enabled: true,
                icon_asset_id: None,
                icon_hash: None,
            },
        )]),
        ..Default::default()
    };

    let plan = build_sync_plan(&config, &lockfile, Path::new(".")).unwrap();
    match &plan.badges[0].action {
        Action::Update { changes } => {
            assert!(changes.iter().any(|c| c.field == "description"));
        }
        other => panic!("expected Update, got {:?}", other),
    }
}

#[test]
fn badge_enabled_change_updates() {
    let config = make_config(
        BTreeMap::new(),
        BTreeMap::from([(
            "Welcome".into(),
            BadgeConfig {
                name: None,
                description: None,
                icon: None,
                enabled: false,
                path: None,
            },
        )]),
        BTreeMap::new(),
    );
    let lockfile = Lockfile {
        badges: BTreeMap::from([(
            "Welcome".into(),
            BadgeLock {
                id: 1,
                name: "Welcome".into(),
                description: None,
                enabled: true,
                icon_asset_id: None,
                icon_hash: None,
            },
        )]),
        ..Default::default()
    };

    let plan = build_sync_plan(&config, &lockfile, Path::new(".")).unwrap();
    match &plan.badges[0].action {
        Action::Update { changes } => {
            assert!(changes.iter().any(|c| c.field == "enabled"));
        }
        other => panic!("expected Update, got {:?}", other),
    }
}

#[test]
fn badge_icon_change_updates() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("badge.png"), b"badge icon data").unwrap();

    let config = make_config(
        BTreeMap::new(),
        BTreeMap::from([(
            "Welcome".into(),
            BadgeConfig {
                name: None,
                description: None,
                icon: Some("badge.png".into()),
                enabled: true,
                path: None,
            },
        )]),
        BTreeMap::new(),
    );
    let lockfile = Lockfile {
        badges: BTreeMap::from([(
            "Welcome".into(),
            BadgeLock {
                id: 1,
                name: "Welcome".into(),
                description: None,
                enabled: true,
                icon_asset_id: Some(100),
                icon_hash: Some("oldhash".into()),
            },
        )]),
        ..Default::default()
    };

    let plan = build_sync_plan(&config, &lockfile, dir.path()).unwrap();
    match &plan.badges[0].action {
        Action::Update { changes } => {
            assert!(changes.iter().any(|c| c.field == "icon"));
        }
        other => panic!("expected Update, got {:?}", other),
    }
}

// --- Product tests ---

#[test]
fn new_product_creates() {
    let config = make_config(
        BTreeMap::new(),
        BTreeMap::new(),
        BTreeMap::from([(
            "Coins".into(),
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
    let lockfile = Lockfile::default();

    let plan = build_sync_plan(&config, &lockfile, Path::new(".")).unwrap();
    assert_eq!(plan.products.len(), 1);
    assert!(matches!(plan.products[0].action, Action::Create));
}

#[test]
fn product_price_change_updates() {
    let config = make_config(
        BTreeMap::new(),
        BTreeMap::new(),
        BTreeMap::from([(
            "Coins".into(),
            ProductConfig {
                name: None,
                price: 199,
                description: None,
                icon: None,
                for_sale: true,
                regional_pricing: false,
                store_page: false,
                path: None,
            },
        )]),
    );
    let lockfile = Lockfile {
        products: BTreeMap::from([(
            "Coins".into(),
            ProductLock {
                id: 1,
                name: "Coins".into(),
                price: 99,
                description: None,
                icon_asset_id: None,
                icon_hash: None,
                for_sale: true,
                regional_pricing: false,
                store_page: false,
            },
        )]),
        ..Default::default()
    };

    let plan = build_sync_plan(&config, &lockfile, Path::new(".")).unwrap();
    match &plan.products[0].action {
        Action::Update { changes } => {
            assert!(changes.iter().any(|c| c.field == "price"));
        }
        other => panic!("expected Update, got {:?}", other),
    }
}

#[test]
fn product_icon_change_updates() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("product.png"), b"product icon data").unwrap();

    let config = make_config(
        BTreeMap::new(),
        BTreeMap::new(),
        BTreeMap::from([(
            "Coins".into(),
            ProductConfig {
                name: None,
                price: 99,
                description: None,
                icon: Some("product.png".into()),
                for_sale: true,
                regional_pricing: false,
                store_page: false,
                path: None,
            },
        )]),
    );
    let lockfile = Lockfile {
        products: BTreeMap::from([(
            "Coins".into(),
            ProductLock {
                id: 1,
                name: "Coins".into(),
                price: 99,
                description: None,
                icon_asset_id: Some(100),
                icon_hash: Some("oldhash".into()),
                for_sale: true,
                regional_pricing: false,
                store_page: false,
            },
        )]),
        ..Default::default()
    };

    let plan = build_sync_plan(&config, &lockfile, dir.path()).unwrap();
    match &plan.products[0].action {
        Action::Update { changes } => {
            assert!(changes.iter().any(|c| c.field == "icon"));
        }
        other => panic!("expected Update, got {:?}", other),
    }
}

// --- New field tests ---

#[test]
fn pass_name_change_updates() {
    let config = make_config(
        BTreeMap::from([(
            "vip".into(),
            PassConfig {
                name: Some("VIP Pass".into()),
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
    let lockfile = Lockfile {
        passes: BTreeMap::from([(
            "vip".into(),
            PassLock {
                id: 1,
                name: "vip".into(),
                price: Some(499),
                description: None,
                icon_asset_id: None,
                icon_hash: None,
                for_sale: true,
                regional_pricing: false,
            },
        )]),
        ..Default::default()
    };

    let plan = build_sync_plan(&config, &lockfile, Path::new(".")).unwrap();
    match &plan.passes[0].action {
        Action::Update { changes } => {
            let name_change = changes.iter().find(|c| c.field == "name").unwrap();
            assert_eq!(name_change.old, "vip");
            assert_eq!(name_change.new, "VIP Pass");
        }
        other => panic!("expected Update, got {:?}", other),
    }
}

#[test]
fn pass_for_sale_change_updates() {
    let config = make_config(
        BTreeMap::from([(
            "VIP".into(),
            PassConfig {
                name: None,
                price: Some(499),
                description: None,
                icon: None,
                for_sale: false,
                regional_pricing: false,
                path: None,
            },
        )]),
        BTreeMap::new(),
        BTreeMap::new(),
    );
    let lockfile = Lockfile {
        passes: BTreeMap::from([(
            "VIP".into(),
            PassLock {
                id: 1,
                name: "VIP".into(),
                price: Some(499),
                description: None,
                icon_asset_id: None,
                icon_hash: None,
                for_sale: true,
                regional_pricing: false,
            },
        )]),
        ..Default::default()
    };

    let plan = build_sync_plan(&config, &lockfile, Path::new(".")).unwrap();
    match &plan.passes[0].action {
        Action::Update { changes } => {
            assert!(changes.iter().any(|c| c.field == "for_sale"));
        }
        other => panic!("expected Update, got {:?}", other),
    }
}

#[test]
fn pass_regional_pricing_change_updates() {
    let config = make_config(
        BTreeMap::from([(
            "VIP".into(),
            PassConfig {
                name: None,
                price: Some(499),
                description: None,
                icon: None,
                for_sale: true,
                regional_pricing: true,
                path: None,
            },
        )]),
        BTreeMap::new(),
        BTreeMap::new(),
    );
    let lockfile = Lockfile {
        passes: BTreeMap::from([(
            "VIP".into(),
            PassLock {
                id: 1,
                name: "VIP".into(),
                price: Some(499),
                description: None,
                icon_asset_id: None,
                icon_hash: None,
                for_sale: true,
                regional_pricing: false,
            },
        )]),
        ..Default::default()
    };

    let plan = build_sync_plan(&config, &lockfile, Path::new(".")).unwrap();
    match &plan.passes[0].action {
        Action::Update { changes } => {
            assert!(changes.iter().any(|c| c.field == "regional_pricing"));
        }
        other => panic!("expected Update, got {:?}", other),
    }
}

#[test]
fn badge_name_change_updates() {
    let config = make_config(
        BTreeMap::new(),
        BTreeMap::from([(
            "welcome".into(),
            BadgeConfig {
                name: Some("Welcome Badge".into()),
                description: None,
                icon: None,
                enabled: true,
                path: None,
            },
        )]),
        BTreeMap::new(),
    );
    let lockfile = Lockfile {
        badges: BTreeMap::from([(
            "welcome".into(),
            BadgeLock {
                id: 1,
                name: "welcome".into(),
                description: None,
                enabled: true,
                icon_asset_id: None,
                icon_hash: None,
            },
        )]),
        ..Default::default()
    };

    let plan = build_sync_plan(&config, &lockfile, Path::new(".")).unwrap();
    match &plan.badges[0].action {
        Action::Update { changes } => {
            let name_change = changes.iter().find(|c| c.field == "name").unwrap();
            assert_eq!(name_change.old, "welcome");
            assert_eq!(name_change.new, "Welcome Badge");
        }
        other => panic!("expected Update, got {:?}", other),
    }
}

#[test]
fn product_for_sale_change_updates() {
    let config = make_config(
        BTreeMap::new(),
        BTreeMap::new(),
        BTreeMap::from([(
            "Coins".into(),
            ProductConfig {
                name: None,
                price: 99,
                description: None,
                icon: None,
                for_sale: false,
                regional_pricing: false,
                store_page: false,
                path: None,
            },
        )]),
    );
    let lockfile = Lockfile {
        products: BTreeMap::from([(
            "Coins".into(),
            ProductLock {
                id: 1,
                name: "Coins".into(),
                price: 99,
                description: None,
                icon_asset_id: None,
                icon_hash: None,
                for_sale: true,
                regional_pricing: false,
                store_page: false,
            },
        )]),
        ..Default::default()
    };

    let plan = build_sync_plan(&config, &lockfile, Path::new(".")).unwrap();
    match &plan.products[0].action {
        Action::Update { changes } => {
            assert!(changes.iter().any(|c| c.field == "for_sale"));
        }
        other => panic!("expected Update, got {:?}", other),
    }
}

#[test]
fn product_store_page_change_updates() {
    let config = make_config(
        BTreeMap::new(),
        BTreeMap::new(),
        BTreeMap::from([(
            "Coins".into(),
            ProductConfig {
                name: None,
                price: 99,
                description: None,
                icon: None,
                for_sale: true,
                regional_pricing: false,
                store_page: true,
                path: None,
            },
        )]),
    );
    let lockfile = Lockfile {
        products: BTreeMap::from([(
            "Coins".into(),
            ProductLock {
                id: 1,
                name: "Coins".into(),
                price: 99,
                description: None,
                icon_asset_id: None,
                icon_hash: None,
                for_sale: true,
                regional_pricing: false,
                store_page: false,
            },
        )]),
        ..Default::default()
    };

    let plan = build_sync_plan(&config, &lockfile, Path::new(".")).unwrap();
    match &plan.products[0].action {
        Action::Update { changes } => {
            assert!(changes.iter().any(|c| c.field == "store_page"));
        }
        other => panic!("expected Update, got {:?}", other),
    }
}
