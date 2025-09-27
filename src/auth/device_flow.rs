use anyhow::{Context, Result};
use chrono::{Duration, Utc};
use colored::*;
use serde::{Deserialize, Serialize};
use std::time;
use tokio::time::sleep;

use crate::api::ApiClient;
use super::token_store::{AuthType, StoredCredentials, TokenStore};

#[derive(Debug, Serialize)]
pub struct DeviceFlowRequest {
    pub client_id: String,
    pub scope: String,
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
    pub scope: String,
}

#[derive(Debug, Deserialize)]
pub struct DeviceTokenError {
    pub error: String,
    pub error_description: String,
}

pub struct DeviceAuth {
    client: ApiClient,
    store: TokenStore,
}

impl DeviceAuth {
    pub fn new(api_endpoint: String) -> Result<Self> {
        Ok(Self {
            client: ApiClient::new(api_endpoint),
            store: TokenStore::new()?,
        })
    }

    pub async fn authenticate(&self) -> Result<()> {
        println!("{}  Initiating device authentication...", "🔐".cyan());

        // Step 1: Initiate device flow
        let device_flow = self.initiate_device_flow().await?;

        // Display user code and instructions
        println!();
        println!("{}  Please visit: {}", "🌐".blue(), device_flow.verification_uri.bright_cyan());
        println!();
        println!("     And enter this code:");
        println!();
        println!("     {}", device_flow.user_code.bright_green().bold());
        println!();
        println!("{}  Waiting for authorization...", "⏳".yellow());

        // Try to open browser automatically
        if let Err(_) = open::that(&device_flow.verification_uri_complete) {
            // Silently fail if we can't open the browser
        }

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
            client_id: "kanuni-cli".to_string(),
            scope: "full_access".to_string(),
        };

        let response = self
            .client
            .post("/api/v1/auth/device/code")
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
        let max_attempts = 120; // Max 10 minutes with 5 second intervals

        for _ in 0..max_attempts {
            sleep(poll_interval).await;

            let request = DeviceTokenRequest {
                device_code: device_code.clone(),
                client_id: "kanuni-cli".to_string(),
            };

            let response = self
                .client
                .post("/api/v1/auth/device/token")
                .json(&request)
                .send()
                .await?;

            if response.status().is_success() {
                return response
                    .json::<DeviceTokenResponse>()
                    .await
                    .context("Failed to parse token response");
            }

            if let Ok(error) = response.json::<DeviceTokenError>().await {
                match error.error.as_str() {
                    "authorization_pending" => {
                        // Continue polling
                        continue;
                    }
                    "slow_down" => {
                        // Increase polling interval
                        poll_interval = time::Duration::from_secs((interval + 5) as u64);
                        continue;
                    }
                    "access_denied" => {
                        return Err(anyhow::anyhow!("Authorization was denied"));
                    }
                    "expired_token" => {
                        return Err(anyhow::anyhow!("Device code has expired"));
                    }
                    _ => {
                        return Err(anyhow::anyhow!(
                            "Authentication failed: {}",
                            error.error_description
                        ));
                    }
                }
            }
        }

        Err(anyhow::anyhow!("Authentication timeout"))
    }
}