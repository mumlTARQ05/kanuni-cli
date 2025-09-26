use anyhow::{Result, Context};
use colored::*;
use dialoguer::{Input, theme::ColorfulTheme};
use rpassword;
use crate::cli::AuthAction;
use crate::config::Config;
use crate::auth::AuthManager;

pub async fn execute(action: &AuthAction) -> Result<()> {
    let config = Config::load()?;
    let auth_manager = AuthManager::new(config)?;

    match action {
        AuthAction::Login { api_key: _ } => {
            // We're now using email/password auth instead of API keys
            let theme = ColorfulTheme::default();

            // Prompt for email
            let email: String = Input::with_theme(&theme)
                .with_prompt("Email")
                .validate_with(|input: &String| -> Result<(), &str> {
                    if input.contains('@') && input.contains('.') {
                        Ok(())
                    } else {
                        Err("Please enter a valid email address")
                    }
                })
                .interact_text()?;

            // Prompt for password (hidden input)
            print!("Password: ");
            use std::io::{self, Write};
            io::stdout().flush().unwrap();
            let password = rpassword::read_password()
                .context("Failed to read password")?;

            // Initial login attempt
            println!("{}  Authenticating...", "🔐".cyan());
            match auth_manager.login(&email, &password, None).await {
                Ok(_) => {
                    println!("{}  Successfully authenticated!", "✓".green());
                    println!("  Welcome to Kanuni - The Legal Intelligence CLI");
                }
                Err(e) => {
                    let error_msg = e.to_string();
                    if error_msg.contains("MFA") || error_msg.contains("forbidden") {
                        // MFA is required
                        println!("{}  MFA Required", "🔑".yellow());
                        let mfa_code: String = Input::with_theme(&theme)
                            .with_prompt("Enter MFA code")
                            .validate_with(|input: &String| -> Result<(), &str> {
                                if input.len() == 6 && input.chars().all(|c| c.is_numeric()) {
                                    Ok(())
                                } else {
                                    Err("MFA code must be 6 digits")
                                }
                            })
                            .interact_text()?;

                        // Retry with MFA
                        auth_manager.login(&email, &password, Some(mfa_code)).await
                            .context("Authentication failed")?;

                        println!("{}  Successfully authenticated with MFA!", "✓".green());
                        println!("  Welcome to Kanuni - The Legal Intelligence CLI");
                    } else {
                        return Err(e);
                    }
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
    }

    Ok(())
}