use anyhow::{Context, Result};
use chrono::{Duration, Utc};
use colored::*;
use serde::{Deserialize, Serialize};
use std::time;
use tokio::time::sleep;

use super::token_store::{AuthType, StoredCredentials, TokenStore};
use crate::config::Config;
use reqwest::Client;

#[derive(Debug, Serialize)]
pub struct DeviceFlowRequest {
    pub client_id: Option<String>,
    pub scopes: Option<Vec<String>>,
}

#[derive(Debug, Deserialize)]
pub struct DeviceFlowResponse {
    pub device_code: String,
    pub user_code: String,
    pub verification_uri: String,
    pub verification_uri_complete: String,
    pub expires_in: i64,
    pub interval: i64,
}

#[derive(Debug, Serialize)]
pub struct DeviceTokenRequest {
    pub device_code: String,
    pub client_id: String,
}

#[derive(Debug, Deserialize)]
pub struct DeviceTokenResponse {
    pub access_token: String,
    pub refresh_token: String,
    pub token_type: String,
    pub expires_in: i64,
    pub scope: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub struct DeviceTokenError {
    pub error: String,
    pub error_description: String,
}

pub struct DeviceAuth {
    client: Client,
    base_url: String,
    store: TokenStore,
}

impl DeviceAuth {
    pub fn new(config: Config) -> Result<Self> {
        let client = Client::builder()
            .timeout(time::Duration::from_secs(30))
            .build()?;

        Ok(Self {
            client,
            base_url: config.api_endpoint.clone(),
            store: TokenStore::new()?,
        })
    }

    pub async fn authenticate(&self) -> Result<()> {
        println!("{}  Initiating device authentication...", "🔐".cyan());

        // Step 1: Initiate device flow
        let device_flow = self.initiate_device_flow().await?;

        // Display user code and instructions
        println!();

        // Try to open browser automatically
        if open::that(&device_flow.verification_uri_complete).is_ok() {
            println!("{}  Opening browser to authorize device...", "🌐".blue());
            println!();
            println!("     If the browser doesn't open automatically, visit:");
            println!("     {}", device_flow.verification_uri_complete.bright_cyan());
        } else {
            println!(
                "{}  Please visit: {}",
                "🌐".blue(),
                device_flow.verification_uri.bright_cyan()
            );
            println!();
            println!("     And enter this code:");
            println!();
            println!("     {}", device_flow.user_code.bright_green().bold());
            println!();
            println!("     Or visit this URL directly:");
            println!("     {}", device_flow.verification_uri_complete.bright_cyan());
        }

        println!();
        println!("{}  Waiting for authorization...", "⏳".yellow());

        // Step 2: Poll for token
        let token_response = self
            .poll_for_token(device_flow.device_code, device_flow.interval)
            .await?;

        // Step 3: Save credentials
        let expires_at = Utc::now() + Duration::seconds(token_response.expires_in);

        let credentials = StoredCredentials {
            auth_type: AuthType::OAuth {
                access_token: token_response.access_token,
                refresh_token: token_response.refresh_token,
                expires_at,
            },
            user_id: None, // Will be populated on first API call
            email: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        self.store.save_credentials(credentials)?;

        println!("{}  Successfully authenticated!", "✓".green());
        println!("  Welcome to Kanuni - The Legal Intelligence CLI");

        Ok(())
    }

    async fn initiate_device_flow(&self) -> Result<DeviceFlowResponse> {
        let request = DeviceFlowRequest {
            client_id: Some("kanuni-cli".to_string()),
            scopes: Some(vec![
                "read_documents".to_string(),
                "write_documents".to_string(),
                "read_cases".to_string(),
                "write_cases".to_string(),
                "read_chat".to_string(),
                "write_chat".to_string(),
            ]),
        };

        let url = format!("{}/auth/device/code", self.base_url);
        let response = self
            .client
            .post(&url)
            .json(&request)
            .send()
            .await?
            .error_for_status()?
            .json::<DeviceFlowResponse>()
            .await?;

        Ok(response)
    }

    async fn poll_for_token(
        &self,
        device_code: String,
        interval: i64,
    ) -> Result<DeviceTokenResponse> {
        let mut poll_interval = time::Duration::from_secs(interval as u64);
        let max_attempts = 180; // Max 15 minutes (15 * 60 / 5 = 180)
        let mut attempt = 0;

        for _ in 0..max_attempts {
            sleep(poll_interval).await;
            attempt += 1;

            let request = DeviceTokenRequest {
                device_code: device_code.clone(),
                client_id: "kanuni-cli".to_string(),
            };

            let url = format!("{}/auth/device/token", self.base_url);
            let response = self.client.post(&url).json(&request).send().await?;

            if response.status().is_success() {
                return response
                    .json::<DeviceTokenResponse>()
                    .await
                    .context("Failed to parse token response");
            }

            if let Ok(error) = response.json::<DeviceTokenError>().await {
                match error.error.as_str() {
                    "authorization_pending" => {
                        // Show progress indicator every 3 attempts (15 seconds)
                        if attempt % 3 == 0 {
                            print!(".");
                            use std::io::{self, Write};
                            io::stdout().flush().ok();
                        }
                        continue;
                    }
                    "slow_down" => {
                        // Increase polling interval by 5 seconds as per OAuth spec
                        poll_interval = time::Duration::from_secs(poll_interval.as_secs() + 5);
                        println!();
                        println!("{}  Slowing down polling interval...", "⚠️ ".yellow());
                        continue;
                    }
                    "access_denied" => {
                        println!();
                        return Err(anyhow::anyhow!("❌ Authorization was denied by the user"));
                    }
                    "expired_token" => {
                        println!();
                        return Err(anyhow::anyhow!("⏱️  Device code has expired. Please try again."));
                    }
                    _ => {
                        println!();
                        return Err(anyhow::anyhow!(
                            "Authentication failed: {}",
                            error.error_description
                        ));
                    }
                }
            }
        }

        println!();
        Err(anyhow::anyhow!("⏱️  Authentication timeout. Please try again."))
    }
}
