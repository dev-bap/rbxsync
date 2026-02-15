use std::path::Path;

use anyhow::Result;
use colored::Colorize;

use crate::cli::Cli;
use crate::config::Config;
use crate::diff::{build_sync_plan, Action};
use crate::lockfile::Lockfile;

pub async fn run(cli: &Cli) -> Result<()> {
    // Validate config
    let config = Config::load(&cli.config)?;
    println!("{} Config is valid ({})", "✓".green(), cli.config.display());

    let config_dir = cli.config.parent().unwrap_or(Path::new("."));
    let lockfile_path = config_dir.join(crate::lockfile::LOCKFILE_NAME);

    if !lockfile_path.exists() {
        println!(
            "{} No lockfile found. Run `rbxsync sync` to create one.",
            "!".yellow()
        );
        return Ok(());
    }

    let lockfile = Lockfile::load(&lockfile_path)?;
    println!(
        "{} Lockfile is valid ({})",
        "✓".green(),
        lockfile_path.display()
    );

    // Check universe_id match
    if lockfile.universe_id != config.experience.universe_id {
        println!(
            "{} Universe ID mismatch: config={}, lockfile={}",
            "✗".red(),
            config.experience.universe_id,
            lockfile.universe_id
        );
    }

    let plan = build_sync_plan(&config, &lockfile, config_dir)?;

    for warning in &plan.warnings {
        println!("{} {}", "!".yellow(), warning);
    }

    // Count changes
    let mut creates = 0;
    let mut updates = 0;
    for action in plan.passes.iter().chain(&plan.badges).chain(&plan.products) {
        match &action.action {
            Action::Create => creates += 1,
            Action::Update { .. } => updates += 1,
            Action::Skip => {}
        }
    }

    if creates == 0 && updates == 0 {
        println!("{} Everything is in sync.", "✓".green());
    } else {
        println!(
            "{} Out of sync: {} to create, {} to update. Run `rbxsync sync` or `rbxsync diff` for details.",
            "!".yellow(),
            creates,
            updates
        );
    }

    Ok(())
}
