use anyhow::{Context, Result};
use chrono::Utc;
use colored::*;
use comfy_table::{
    modifiers::UTF8_ROUND_CORNERS, presets::UTF8_FULL, Cell, ContentArrangement, Table,
};
use dialoguer::{theme::ColorfulTheme, Confirm, Input};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::token_store::{AuthType, StoredCredentials, TokenStore};
use crate::config::Config;
use reqwest::{Client, StatusCode};

#[derive(Debug, Serialize)]
pub struct CreateApiKeyRequest {
    pub name: String,
    pub permissions: Vec<String>,
    pub expires_in_days: Option<i64>,
}

#[derive(Debug, Deserialize)]
pub struct CreateApiKeyResponse {
    pub key_id: Uuid,
    pub name: String,
    pub api_key: String,
    pub prefix: String,
    pub last_4: String,
    pub permissions: Vec<String>,
    pub expires_at: Option<chrono::DateTime<chrono::Utc>>,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Deserialize)]
pub struct ApiKeyInfo {
    pub id: Uuid,
    pub name: String,
    pub prefix: String,
    pub last_4: String,
    pub permissions: Vec<String>,
    pub last_used_at: Option<chrono::DateTime<chrono::Utc>>,
    pub expires_at: Option<chrono::DateTime<chrono::Utc>>,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

pub struct ApiKeyManager {
    client: Client,
    base_url: String,
    store: TokenStore,
}

impl ApiKeyManager {
    pub fn new(config: Config) -> Result<Self> {
        let client = Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()?;

        Ok(Self {
            client,
            base_url: config.api_endpoint.clone(),
            store: TokenStore::new()?,
        })
    }

    pub async fn create_key(&self, access_token: &str) -> Result<()> {
        let theme = ColorfulTheme::default();

        // Prompt for key name
        let name: String = Input::with_theme(&theme)
            .with_prompt("API Key name")
            .default("CLI Key".to_string())
            .interact_text()?;

        // Prompt for expiration
        let expires_in_days: String = Input::with_theme(&theme)
            .with_prompt("Expiration (days, or 'never')")
            .default("never".to_string())
            .interact_text()?;

        let expires_in_days = if expires_in_days.to_lowercase() == "never" {
            None
        } else {
            expires_in_days.parse::<i64>().ok()
        };

        // Default permissions
        let permissions = vec!["read".to_string(), "write".to_string()];

        println!("{}  Creating API key...", "🔑".yellow());

        let request = CreateApiKeyRequest {
            name: name.clone(),
            permissions: permissions.clone(),
            expires_in_days,
        };

        let url = format!("{}/api/v1/account/api-keys", self.base_url);
        let response = self
            .client
            .post(&url)
            .header("Authorization", format!("Bearer {}", access_token))
            .json(&request)
            .send()
            .await?
            .error_for_status()?
            .json::<CreateApiKeyResponse>()
            .await?;

        println!();
        println!("{}  API Key created successfully!", "✓".green());
        println!();
        println!("  {}: {}", "Name".bright_blue(), response.name);
        println!(
            "  {}: {}...{}",
            "Identifier".bright_blue(),
            response.prefix,
            response.last_4
        );
        if let Some(expires) = response.expires_at {
            println!(
                "  {}: {}",
                "Expires".bright_blue(),
                expires.format("%Y-%m-%d %H:%M:%S UTC")
            );
        }
        println!();
        println!(
            "  {}:",
            "Your API Key (SAVE THIS NOW - IT WON'T BE SHOWN AGAIN)"
                .bright_red()
                .bold()
        );
        println!();
        println!("    {}", response.api_key.bright_green());
        println!();
        println!(
            "  Use this key with the {} header or as:",
            "X-API-Key".cyan()
        );
        println!("    kanuni auth login --api-key {}", response.api_key);
        println!();

        // Ask if user wants to use this key now
        if Confirm::with_theme(&theme)
            .with_prompt("Use this API key for CLI authentication now?")
            .default(true)
            .interact()?
        {
            self.authenticate_with_key(
                response.api_key,
                response.name,
                response.prefix,
                response.last_4,
            )
            .await?;
        }

        Ok(())
    }

    pub async fn authenticate_with_key(
        &self,
        api_key: String,
        name: String,
        prefix: String,
        last_4: String,
    ) -> Result<()> {
        // Test the API key by making a request
        let url = format!("{}/api/v1/auth/profile", self.base_url);
        let response = self
            .client
            .get(&url)
            .header("X-API-Key", &api_key)
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(anyhow::anyhow!("Invalid API key"));
        }

        // Parse user info
        let user_info: serde_json::Value = response.json().await?;

        let credentials = StoredCredentials {
            auth_type: AuthType::ApiKey {
                key: api_key,
                name,
                prefix,
                last_4,
            },
            user_id: user_info
                .get("id")
                .and_then(|v| v.as_str())
                .and_then(|s| Uuid::parse_str(s).ok()),
            email: user_info
                .get("email")
                .and_then(|v| v.as_str())
                .map(String::from),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        self.store.save_credentials(credentials)?;

        println!("{}  Successfully authenticated with API key!", "✓".green());

        Ok(())
    }

    pub async fn list_keys(&self, access_token: &str) -> Result<()> {
        let url = format!("{}/api/v1/account/api-keys", self.base_url);
        let keys = self
            .client
            .get(&url)
            .header("Authorization", format!("Bearer {}", access_token))
            .send()
            .await?
            .error_for_status()?
            .json::<Vec<ApiKeyInfo>>()
            .await?;

        if keys.is_empty() {
            println!("No API keys found.");
            return Ok(());
        }

        let mut table = Table::new();
        table
            .load_preset(UTF8_FULL)
            .apply_modifier(UTF8_ROUND_CORNERS)
            .set_content_arrangement(ContentArrangement::Dynamic)
            .set_header(vec![
                Cell::new("Name").add_attribute(comfy_table::Attribute::Bold),
                Cell::new("Key ID").add_attribute(comfy_table::Attribute::Bold),
                Cell::new("Last Used").add_attribute(comfy_table::Attribute::Bold),
                Cell::new("Expires").add_attribute(comfy_table::Attribute::Bold),
                Cell::new("Created").add_attribute(comfy_table::Attribute::Bold),
            ]);

        for key in keys {
            let key_id = format!("{}...{}", key.prefix, key.last_4);
            let last_used = key
                .last_used_at
                .map(|dt| dt.format("%Y-%m-%d").to_string())
                .unwrap_or_else(|| "Never".to_string());
            let expires = key
                .expires_at
                .map(|dt| dt.format("%Y-%m-%d").to_string())
                .unwrap_or_else(|| "Never".to_string());
            let created = key.created_at.format("%Y-%m-%d").to_string();

            table.add_row(vec![key.name, key_id, last_used, expires, created]);
        }

        println!("{table}");
        Ok(())
    }

    pub async fn revoke_key(&self, access_token: &str, key_id: Uuid) -> Result<()> {
        let theme = ColorfulTheme::default();

        if !Confirm::with_theme(&theme)
            .with_prompt("Are you sure you want to revoke this API key?")
            .default(false)
            .interact()?
        {
            println!("Cancelled.");
            return Ok(());
        }

        let url = format!("{}/api/v1/account/api-keys/{}", self.base_url, key_id);
        self.client
            .delete(&url)
            .header("Authorization", format!("Bearer {}", access_token))
            .send()
            .await?
            .error_for_status()?;

        println!("{}  API key revoked successfully!", "✓".green());

        Ok(())
    }
}
