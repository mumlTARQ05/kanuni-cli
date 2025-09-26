mod api;
mod auth;
mod cli;
mod commands;
mod config;
mod utils;

use anyhow::Result;
use clap::Parser;
use colored::*;
use tracing_subscriber;

use crate::cli::Cli;
use crate::utils::banner;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("warn"))
        )
        .init();

    // Display banner
    banner::display();

    // Parse CLI arguments
    let cli = Cli::parse();

    // Execute command
    match cli.execute().await {
        Ok(_) => Ok(()),
        Err(e) => {
            eprintln!("{} {}", "Error:".red().bold(), e);
            std::process::exit(1);
        }
    }
}
