use std::path::PathBuf;

use clap::{Parser, Subcommand, ValueEnum};

#[derive(Parser)]
#[command(
    name = "rbxsync",
    about = "Declaratively manage Roblox game passes, badges, and developer products"
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,

    /// Path to config file
    #[arg(long, global = true, default_value = "rbxsync.toml")]
    pub config: PathBuf,

    /// Roblox Open Cloud API key
    #[arg(
        long,
        global = true,
        long_help = "\
Roblox Open Cloud API key.
Create one at: https://create.roblox.com/dashboard/credentials

Required API scopes:

  Game Passes
    game-pass:read, game-pass:write
    https://create.roblox.com/docs/cloud/api/game-passes

  Developer Products
    developer-product:read, developer-product:write
    https://create.roblox.com/docs/cloud/api/developer-products

  Badges
    legacy-universe.badge:read, legacy-universe.badge:write, legacy-universe.badge:manage-and-spend-robux
    https://create.roblox.com/docs/cloud/api/badges
    https://create.roblox.com/docs/cloud/features/universes#badges

  Assets (icon downloads)
    legacy-asset:manage
    https://create.roblox.com/docs/cloud/features/assets#/"
    )]
    pub api_key: Option<String>,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Initialize a new rbxsync.toml config file
    Init {
        /// Populate config from existing remote resources
        #[arg(long)]
        from_remote: bool,

        /// Universe ID (required with --from-remote)
        #[arg(long)]
        universe_id: Option<u64>,
    },

    /// Sync local config to Roblox
    Sync {
        /// Show what would change without applying
        #[arg(long)]
        dry_run: bool,

        /// Only sync specific resource types (comma-separated)
        #[arg(long, value_delimiter = ',')]
        only: Option<Vec<ResourceType>>,

        /// Expected cost in Robux when creating a badge (default: 0)
        #[arg(long, default_value_t = 0)]
        badge_cost: u64,
    },

    /// List remote resources (passes, badges, products)
    List {
        /// Resource type to list
        resource: ResourceType,
    },

    /// Check config validity and diff against lockfile
    Check,

    /// Pull remote state into lockfile
    Pull {
        /// Show what remote state differs without writing anything
        #[arg(long)]
        dry_run: bool,

        /// Keep remote icons (set local hash so next sync skips re-upload)
        #[arg(long, conflicts_with = "accept_local")]
        accept_remote: bool,

        /// Re-upload local icons on next sync
        #[arg(long, conflicts_with = "accept_remote")]
        accept_local: bool,
    },

    /// Rename a resource key in config and lockfile
    Rename {
        /// Resource type
        resource: ResourceType,
        /// Current key name
        old_key: String,
        /// New key name
        new_key: String,
    },
}

#[derive(Clone, Debug, ValueEnum, PartialEq, Eq)]
pub enum ResourceType {
    Passes,
    Badges,
    Products,
}
