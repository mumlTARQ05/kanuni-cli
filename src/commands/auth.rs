use anyhow::{Result, Context};
use colored::*;
use dialoguer::{Select, theme::ColorfulTheme};
use crate::cli::AuthAction;
use crate::config::Config;
use crate::auth::AuthManager;

pub async fn execute(action: &AuthAction) -> Result<()> {
    let config = Config::load()?;
    let auth_manager = AuthManager::new(config)?;

    match action {
        AuthAction::Login { api_key } => {
            if let Some(key) = api_key {
                // Direct API key authentication
                println!("{}  Authenticating with API key...", "🔑".cyan());
                auth_manager.login_api_key(key.clone()).await?;
                println!("{}  Successfully authenticated!", "✓".green());
                println!("  Welcome to Kanuni - The Legal Intelligence CLI");
            } else {
                // Let user choose authentication method
                let theme = ColorfulTheme::default();
                let items = vec![
                    "Browser Authentication (Recommended)",
                    "API Key",
                ];

                let selection = Select::with_theme(&theme)
                    .with_prompt("Choose authentication method")
                    .items(&items)
                    .default(0)
                    .interact()?;

                match selection {
                    0 => {
                        // Device flow authentication
                        auth_manager.login_device_flow().await?;
                    }
                    1 => {
                        // Prompt for API key
                        println!("Enter your API key (or run 'kanuni auth create-key' to generate one):");
                        let mut api_key = String::new();
                        std::io::stdin().read_line(&mut api_key)?;
                        let api_key = api_key.trim().to_string();

                        println!("{}  Authenticating with API key...", "🔑".cyan());
                        auth_manager.login_api_key(api_key).await?;
                        println!("{}  Successfully authenticated!", "✓".green());
                        println!("  Welcome to Kanuni - The Legal Intelligence CLI");
                    }
                    _ => unreachable!(),
                }
            }
        }
        AuthAction::Logout => {
            auth_manager.logout().await?;
            println!("{}  Successfully logged out", "✓".green());
        }
        AuthAction::Status => {
            let status = auth_manager.status().await?;

            if auth_manager.is_authenticated().await {
                println!("{}  Authentication Status: {}", "🔐".green(), "AUTHENTICATED".green().bold());
                println!("{}", status);
            } else {
                println!("{}  Authentication Status: {}", "🔐".red(), "NOT AUTHENTICATED".red().bold());
                println!("  Run {} to authenticate", "kanuni auth login".cyan());
            }
        }
        AuthAction::CreateKey => {
            if !auth_manager.is_authenticated().await {
                println!("{}  You must be authenticated to create API keys", "⚠️".yellow());
                println!("  Run {} first", "kanuni auth login".cyan());
                return Ok(());
            }

            auth_manager.create_api_key().await?;
        }
        AuthAction::ListKeys => {
            if !auth_manager.is_authenticated().await {
                println!("{}  You must be authenticated to list API keys", "⚠️".yellow());
                println!("  Run {} first", "kanuni auth login".cyan());
                return Ok(());
            }

            auth_manager.list_api_keys().await?;
        }
    }

    Ok(())
}