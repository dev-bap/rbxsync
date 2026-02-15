use anyhow::Result;
use clap::Parser;
use rbxsync::cli::{Cli, Commands};
use rbxsync::commands;

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    match &cli.command {
        Commands::Init {
            from_remote,
            universe_id,
        } => commands::init::run(&cli, *from_remote, *universe_id).await,
        Commands::Sync {
            dry_run,
            only,
            badge_cost,
        } => commands::sync::run(&cli, *dry_run, only.clone(), *badge_cost).await,
        Commands::List { resource } => commands::list::run(&cli, resource.clone()).await,
        Commands::Check => commands::check::run(&cli).await,
        Commands::Pull {
            dry_run,
            accept_remote,
            accept_local,
        } => commands::pull::run(&cli, *dry_run, *accept_remote, *accept_local).await,
        Commands::Rename {
            resource,
            old_key,
            new_key,
        } => commands::rename::run(&cli, resource.clone(), old_key, new_key),
    }
}
