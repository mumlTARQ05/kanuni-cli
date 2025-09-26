pub mod client;
pub mod models;
pub mod storage;

use anyhow::{Result, Context};
use chrono::{DateTime, Utc, Duration};
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::config::Config;
use self::{
    client::AuthClient,
    models::{AuthTokens, LoginRequest, LoginResponse, RefreshRequest, UserInfo},
    storage::TokenStorage,
};

pub struct AuthManager {
    client: AuthClient,
    storage: TokenStorage,
    tokens: Arc<RwLock<Option<AuthTokens>>>,
    config: Arc<RwLock<Config>>,
}

impl AuthManager {
    pub fn new(config: Config) -> Result<Self> {
        let api_endpoint = config.api_endpoint.clone();
        let storage = TokenStorage::new()?;

        Ok(Self {
            client: AuthClient::new(api_endpoint),
            storage,
            tokens: Arc::new(RwLock::new(None)),
            config: Arc::new(RwLock::new(config)),
        })
    }

    pub async fn login(&self, email: &str, password: &str, mfa_code: Option<String>) -> Result<()> {
        // Create login request
        let request = LoginRequest {
            email: email.to_string(),
            password: password.to_string(),
            mfa_code,
        };

        // Send login request
        let response = self.client.login(request).await
            .context("Failed to authenticate with server")?;

        // Store tokens
        self.storage.store_refresh_token(&response.refresh_token)?;

        // Store access token in memory
        let tokens = AuthTokens {
            access_token: response.access_token,
            refresh_token: response.refresh_token,
            expires_at: Utc::now() + Duration::seconds(response.expires_in),
            user: response.user,
        };

        *self.tokens.write().await = Some(tokens.clone());

        // Update config with user email
        let mut config = self.config.write().await;
        config.user_email = Some(email.to_string());
        config.save()?;

        Ok(())
    }

    pub async fn logout(&self) -> Result<()> {
        // Clear tokens from storage
        self.storage.clear_tokens()?;

        // Clear from memory
        *self.tokens.write().await = None;

        // Clear user email from config
        let mut config = self.config.write().await;
        config.user_email = None;
        config.save()?;

        // Optional: Call logout endpoint
        // self.client.logout().await?;

        Ok(())
    }

    pub async fn get_access_token(&self) -> Result<String> {
        // Check if we have tokens in memory
        let mut tokens_guard = self.tokens.write().await;

        if let Some(tokens) = tokens_guard.as_ref() {
            // Check if token is expired or about to expire (5 minutes buffer)
            if tokens.expires_at > Utc::now() + Duration::minutes(5) {
                return Ok(tokens.access_token.clone());
            }
        }

        // Try to refresh the token
        let refresh_token = self.storage.get_refresh_token()
            .context("No refresh token found. Please login first.")?;

        let response = self.client
            .refresh_token(RefreshRequest { refresh_token: refresh_token.clone() })
            .await
            .context("Failed to refresh token. Please login again.")?;

        // Update tokens
        let tokens = AuthTokens {
            access_token: response.access_token.clone(),
            refresh_token: refresh_token, // Keep the same refresh token
            expires_at: Utc::now() + Duration::seconds(response.expires_in),
            user: response.user,
        };

        *tokens_guard = Some(tokens);

        Ok(response.access_token)
    }

    pub async fn get_user_info(&self) -> Result<Option<UserInfo>> {
        let tokens = self.tokens.read().await;
        Ok(tokens.as_ref().map(|t| t.user.clone()))
    }

    pub async fn is_authenticated(&self) -> bool {
        // Check if we have a refresh token stored
        self.storage.get_refresh_token().is_ok()
    }

    pub async fn status(&self) -> Result<String> {
        if !self.is_authenticated().await {
            return Ok("Not authenticated".to_string());
        }

        let user_info = self.get_user_info().await?;
        let config = self.config.read().await;

        if let Some(user) = user_info {
            Ok(format!(
                "Authenticated as: {}\nSubscription: {}\nAPI Endpoint: {}",
                user.email,
                user.subscription_tier.unwrap_or_else(|| "Free".to_string()),
                config.api_endpoint
            ))
        } else {
            Ok("Authenticated (fetching user info...)".to_string())
        }
    }
}