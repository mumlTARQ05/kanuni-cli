use crate::cli::ConfigAction;
use crate::config::Config;
use anyhow::Result;
use colored::*;

pub async fn execute(action: &ConfigAction) -> Result<()> {
    match action {
        ConfigAction::Show => {
            println!("{}  Current Configuration:", "⚙️".cyan());

            let config = Config::load()?;

            println!(
                "\n  {} {}",
                "API Endpoint:".white().bold(),
                config.api_endpoint.yellow()
            );
            println!(
                "  {} {}",
                "User:".white().bold(),
                config
                    .user_email
                    .as_deref()
                    .unwrap_or("[NOT LOGGED IN]")
                    .yellow()
            );
            println!(
                "  {} {}",
                "Default Format:".white().bold(),
                config.default_format.yellow()
            );
            println!(
                "  {} {}",
                "Color Output:".white().bold(),
                if config.color_output {
                    "enabled".green()
                } else {
                    "disabled".red()
                }
            );
            println!(
                "  {} {}",
                "Verbose:".white().bold(),
                if config.verbose {
                    "true".green()
                } else {
                    "false".white()
                }
            );

            println!(
                "\n  Config file: {}",
                Config::get_config_path()?.display().to_string().blue()
            );
        }
        ConfigAction::Set { key, value } => {
            let mut config = Config::load()?;

            match key.as_str() {
                "api_endpoint" => config.api_endpoint = value.clone(),
                "api_key" => config.api_key = Some(value.clone()),
                "default_format" => config.default_format = value.clone(),
                "color_output" => config.color_output = value.parse()?,
                "verbose" => config.verbose = value.parse()?,
                _ => anyhow::bail!("Unknown configuration key: {}", key),
            }

            config.save()?;
            println!(
                "{}  Configuration updated: {} = {}",
                "✓".green(),
                key.cyan(),
                value.yellow()
            );
        }
        ConfigAction::Reset => {
            Config::reset()?;
            println!("{}  Configuration reset to defaults", "✓".green());
        }
    }

    Ok(())
}
