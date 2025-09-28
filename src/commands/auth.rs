use crate::auth::{AuthManager, sessions::format_session_display};
use crate::cli::{AuthAction, SessionAction};
use crate::config::Config;
use anyhow::{Context, Result};
use colored::*;
use dialoguer::{theme::ColorfulTheme, Select, Confirm};

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
                let items = vec!["Browser Authentication (Recommended)", "API Key"];

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
                        println!(
                            "Enter your API key (or run 'kanuni auth create-key' to generate one):"
                        );
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
                println!(
                    "{}  Authentication Status: {}",
                    "🔐".green(),
                    "AUTHENTICATED".green().bold()
                );
                println!("{}", status);
            } else {
                println!(
                    "{}  Authentication Status: {}",
                    "🔐".red(),
                    "NOT AUTHENTICATED".red().bold()
                );
                println!("  Run {} to authenticate", "kanuni auth login".cyan());
            }
        }
        AuthAction::CreateKey => {
            if !auth_manager.is_authenticated().await {
                println!(
                    "{}  You must be authenticated to create API keys",
                    "⚠️".yellow()
                );
                println!("  Run {} first", "kanuni auth login".cyan());
                return Ok(());
            }

            auth_manager.create_api_key().await?;
        }
        AuthAction::ListKeys => {
            if !auth_manager.is_authenticated().await {
                println!(
                    "{}  You must be authenticated to list API keys",
                    "⚠️".yellow()
                );
                println!("  Run {} first", "kanuni auth login".cyan());
                return Ok(());
            }

            auth_manager.list_api_keys().await?;
        }
        AuthAction::Sessions { action } => {
            if !auth_manager.is_authenticated().await {
                println!(
                    "{}  You must be authenticated to manage CLI sessions",
                    "⚠️".yellow()
                );
                println!("  Run {} first", "kanuni auth login".cyan());
                return Ok(());
            }

            handle_session_action(&auth_manager, action).await?;
        }
    }

    Ok(())
}

async fn handle_session_action(auth_manager: &AuthManager, action: &SessionAction) -> Result<()> {
    match action {
        SessionAction::List => {
            let sessions = auth_manager.list_sessions().await?;
            format_session_display(&sessions);
        }
        SessionAction::Revoke { id } => {
            // Confirm before revoking
            let theme = ColorfulTheme::default();
            let confirm = Confirm::with_theme(&theme)
                .with_prompt("Are you sure you want to revoke this CLI session?")
                .interact()?;

            if confirm {
                auth_manager.revoke_session(id).await?;
                println!("{}  Session revoked successfully", "✓".green());
            } else {
                println!("{}  Cancelled", "ℹ".blue());
            }
        }
        SessionAction::RevokeAll => {
            println!(
                "{}  This will revoke all CLI sessions and log you out!",
                "⚠️".yellow().bold()
            );

            let theme = ColorfulTheme::default();
            let confirm = Confirm::with_theme(&theme)
                .with_prompt("Are you absolutely sure you want to revoke ALL CLI sessions?")
                .default(false)
                .interact()?;

            if confirm {
                let double_check = Confirm::with_theme(&theme)
                    .with_prompt("This action cannot be undone. Continue?")
                    .default(false)
                    .interact()?;

                if double_check {
                    auth_manager.revoke_all_sessions().await?;
                    println!("{}  All CLI sessions revoked", "✓".green());
                    println!("  You have been logged out");
                } else {
                    println!("{}  Cancelled", "ℹ".blue());
                }
            } else {
                println!("{}  Cancelled", "ℹ".blue());
            }
        }
    }

    Ok(())
}
