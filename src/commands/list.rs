use anyhow::Result;
use colored::Colorize;

use crate::api::RbxClient;
use crate::cli::{Cli, ResourceType};
use crate::config::Config;

pub async fn run(cli: &Cli, resource: ResourceType) -> Result<()> {
    let config = Config::load(&cli.config)?;
    let client = RbxClient::new(
        cli.api_key.clone(),
        config.experience.universe_id,
        config.icons.bleed,
    );

    match resource {
        ResourceType::Passes => {
            let passes = client.list_all_game_passes().await?;
            println!("{}", "Game Passes".bold());
            println!("{:<12} {:<30} {:<10} Description", "ID", "Name", "Price");
            println!("{}", "-".repeat(70));
            for pass in &passes {
                let id = pass
                    .id
                    .map(|i| i.to_string())
                    .unwrap_or_else(|| "-".to_string());
                let name = pass.name.as_deref().unwrap_or("-");
                let price = pass
                    .price()
                    .map(|p| format!("R${}", p))
                    .unwrap_or_else(|| "Free".to_string());
                let desc = pass.description.as_deref().unwrap_or("");
                println!("{:<12} {:<30} {:<10} {}", id, name, price, desc);
            }
            println!("\nTotal: {}", passes.len());
        }
        ResourceType::Badges => {
            let badges = client
                .list_all_badges(config.experience.universe_id)
                .await?;
            println!("{}", "Badges".bold());
            println!("{:<12} {:<30} {:<10} Description", "ID", "Name", "Enabled");
            println!("{}", "-".repeat(70));
            for badge in &badges {
                let id = badge
                    .id
                    .map(|i| i.to_string())
                    .unwrap_or_else(|| "-".to_string());
                let name = badge.name.as_deref().unwrap_or("-");
                let enabled = badge
                    .enabled
                    .map(|e| if e { "Yes" } else { "No" })
                    .unwrap_or("-");
                let desc = badge.description.as_deref().unwrap_or("");
                println!("{:<12} {:<30} {:<10} {}", id, name, enabled, desc);
            }
            println!("\nTotal: {}", badges.len());
        }
        ResourceType::Products => {
            let products = client.list_all_developer_products().await?;
            println!("{}", "Developer Products".bold());
            println!("{:<12} {:<30} {:<10} Description", "ID", "Name", "Price");
            println!("{}", "-".repeat(70));
            for product in &products {
                let id = product
                    .id
                    .map(|i| i.to_string())
                    .unwrap_or_else(|| "-".to_string());
                let name = product.name.as_deref().unwrap_or("-");
                let price = product
                    .price()
                    .map(|p| format!("R${}", p))
                    .unwrap_or_else(|| "-".to_string());
                let desc = product.description.as_deref().unwrap_or("");
                println!("{:<12} {:<30} {:<10} {}", id, name, price, desc);
            }
            println!("\nTotal: {}", products.len());
        }
    }

    Ok(())
}
