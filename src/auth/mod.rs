pub mod api_key;
pub mod client;
pub mod device_flow;
pub mod models;
pub mod token_store;

use anyhow::{Context, Result};
use chrono::{Duration, Utc};
use std::sync::Arc;
use tokio::sync::RwLock;

use self::{
    api_key::ApiKeyManager,
    client::AuthClient,
    device_flow::DeviceAuth,
    models::{AuthTokens, RefreshRequest, UserInfo},
    token_store::{AuthType, StoredCredentials, TokenStore},
};
use crate::config::Config;

pub struct AuthManager {
    client: AuthClient,
    store: TokenStore,
    credentials: Arc<RwLock<Option<StoredCredentials>>>,
    config: Arc<RwLock<Config>>,
}

impl AuthManager {
    pub fn new(config: Config) -> Result<Self> {
        let api_endpoint = config.api_endpoint.clone();
        let store = TokenStore::new()?;

        // Load existing credentials
        let credentials = store.load_credentials()?;

        Ok(Self {
            client: AuthClient::new(api_endpoint),
            store,
            credentials: Arc::new(RwLock::new(credentials)),
            config: Arc::new(RwLock::new(config)),
        })
    }

    /// Authenticate using device flow (OAuth)
    pub async fn login_device_flow(&self) -> Result<()> {
        let config = self.config.read().await;
        let device_auth = DeviceAuth::new(config.api_endpoint.clone())?;
        device_auth.authenticate().await?;

        // Reload credentials
        *self.credentials.write().await = self.store.load_credentials()?;

        Ok(())
    }

    /// Authenticate using API key
    pub async fn login_api_key(&self, api_key: String) -> Result<()> {
        // Extract prefix and last 4 from key
        let prefix = if api_key.starts_with("kanuni_live_") {
            "kanuni_live_"
        } else if api_key.starts_with("kanuni_test_") {
            "kanuni_test_"
        } else {
            return Err(anyhow::anyhow!("Invalid API key format"));
        };

        let key_suffix = &api_key[prefix.len()..];
        let last_4 = if key_suffix.len() >= 4 {
            &key_suffix[key_suffix.len() - 4..]
        } else {
            return Err(anyhow::anyhow!("Invalid API key format"));
        };

        let config = self.config.read().await;
        let manager = ApiKeyManager::new(config.api_endpoint.clone())?;

        manager
            .authenticate_with_key(
                api_key,
                "CLI Key".to_string(),
                prefix.to_string(),
                last_4.to_string(),
            )
            .await?;

        // Reload credentials
        *self.credentials.write().await = self.store.load_credentials()?;

        Ok(())
    }

    /// Create a new API key
    pub async fn create_api_key(&self) -> Result<()> {
        let access_token = self.get_access_token().await?;
        let config = self.config.read().await;
        let manager = ApiKeyManager::new(config.api_endpoint.clone())?;
        manager.create_key(&access_token).await?;

        // Reload credentials if user chose to use the new key
        *self.credentials.write().await = self.store.load_credentials()?;

        Ok(())
    }

    /// List API keys
    pub async fn list_api_keys(&self) -> Result<()> {
        let access_token = self.get_access_token().await?;
        let config = self.config.read().await;
        let manager = ApiKeyManager::new(config.api_endpoint.clone())?;
        manager.list_keys(&access_token).await
    }

    /// Logout and clear credentials
    pub async fn logout(&self) -> Result<()> {
        self.store.clear_credentials()?;
        *self.credentials.write().await = None;

        // Clear user email from config
        let mut config = self.config.write().await;
        config.user_email = None;
        config.save()?;

        Ok(())
    }

    /// Get access token (handles refresh for OAuth)
    pub async fn get_access_token(&self) -> Result<String> {
        let credentials_guard = self.credentials.read().await;

        let credentials = credentials_guard
            .as_ref()
            .context("Not authenticated. Please run 'kanuni auth login'")?;

        match &credentials.auth_type {
            AuthType::ApiKey { key, .. } => {
                // API keys don't expire, return as-is
                Ok(key.clone())
            }
            AuthType::OAuth {
                access_token,
                refresh_token,
                expires_at,
            } => {
                // Check if token is expired or about to expire (5 minutes buffer)
                if *expires_at > Utc::now() + Duration::minutes(5) {
                    return Ok(access_token.clone());
                }

                // Need to refresh the token
                drop(credentials_guard); // Release read lock

                let response = self
                    .client
                    .refresh_token(RefreshRequest {
                        refresh_token: refresh_token.clone(),
                    })
                    .await
                    .context("Failed to refresh token. Please login again.")?;

                let new_expires_at = Utc::now() + Duration::seconds(response.expires_in);

                // Update stored tokens
                self.store.update_oauth_tokens(
                    response.access_token.clone(),
                    refresh_token.clone(), // Keep same refresh token
                    new_expires_at,
                )?;

                // Update in-memory credentials
                *self.credentials.write().await = self.store.load_credentials()?;

                Ok(response.access_token)
            }
        }
    }

    /// Check if authenticated
    pub async fn is_authenticated(&self) -> bool {
        self.credentials.read().await.is_some()
    }

    /// Get authentication status
    pub async fn status(&self) -> Result<String> {
        let credentials = self.credentials.read().await;

        if let Some(creds) = credentials.as_ref() {
            let mut status = String::from("Authenticated\n");

            match &creds.auth_type {
                AuthType::ApiKey {
                    name,
                    prefix,
                    last_4,
                    ..
                } => {
                    status.push_str(&format!("  Type: API Key\n"));
                    status.push_str(&format!("  Name: {}\n", name));
                    status.push_str(&format!("  Key: {}...{}\n", prefix, last_4));
                }
                AuthType::OAuth { expires_at, .. } => {
                    status.push_str(&format!("  Type: OAuth (Device Flow)\n"));
                    status.push_str(&format!(
                        "  Token expires: {}\n",
                        expires_at.format("%Y-%m-%d %H:%M:%S UTC")
                    ));
                }
            }

            if let Some(email) = &creds.email {
                status.push_str(&format!("  Email: {}\n", email));
            }

            let config = self.config.read().await;
            status.push_str(&format!("  API Endpoint: {}", config.api_endpoint));

            Ok(status)
        } else {
            Ok("Not authenticated".to_string())
        }
    }
}
