use anyhow::{Result, Context, bail};
use reqwest::{Client, StatusCode};
use serde_json::json;

use super::models::{LoginRequest, LoginResponse, RefreshRequest, RefreshResponse, ErrorResponse};

pub struct AuthClient {
    client: Client,
    base_url: String,
}

impl AuthClient {
    pub fn new(base_url: String) -> Self {
        let client = Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .expect("Failed to create HTTP client");

        Self { client, base_url }
    }

    pub async fn login(&self, request: LoginRequest) -> Result<LoginResponse> {
        let url = format!("{}/auth/login", self.base_url);

        tracing::info!("Attempting login to: {}", url);
        tracing::debug!("Login request: {:?}", request);

        let response = self.client
            .post(&url)
            .json(&request)
            .send()
            .await
            .context(format!("Failed to send login request to {}", url))?;

        match response.status() {
            StatusCode::OK => {
                let login_response = response
                    .json::<LoginResponse>()
                    .await
                    .context("Failed to parse login response")?;
                Ok(login_response)
            }
            StatusCode::UNAUTHORIZED => {
                bail!("Invalid email or password")
            }
            StatusCode::FORBIDDEN => {
                bail!("MFA code required or invalid")
            }
            status => {
                // Try to get response body for debugging
                let body = response.text().await.unwrap_or_else(|_| "No body".to_string());
                tracing::error!("Login failed with status {}: {}", status, body);
                bail!("Login failed with status {}: {}", status, body)
            }
        }
    }

    pub async fn refresh_token(&self, request: RefreshRequest) -> Result<RefreshResponse> {
        let url = format!("{}/auth/refresh", self.base_url);

        let response = self.client
            .post(&url)
            .json(&request)
            .send()
            .await
            .context("Failed to send refresh token request")?;

        match response.status() {
            StatusCode::OK => {
                let refresh_response = response
                    .json::<RefreshResponse>()
                    .await
                    .context("Failed to parse refresh token response")?;
                Ok(refresh_response)
            }
            StatusCode::UNAUTHORIZED => {
                bail!("Refresh token is invalid or expired. Please login again.")
            }
            status => {
                if let Ok(error) = response.json::<ErrorResponse>().await {
                    bail!("{}: {}", status, error.message)
                } else {
                    bail!("Token refresh failed with status: {}", status)
                }
            }
        }
    }

    pub async fn logout(&self, token: &str) -> Result<()> {
        let url = format!("{}/auth/logout", self.base_url);

        let response = self.client
            .post(&url)
            .header("Authorization", format!("Bearer {}", token))
            .send()
            .await
            .context("Failed to send logout request")?;

        if !response.status().is_success() {
            tracing::warn!("Logout request failed, but continuing with local logout");
        }

        Ok(())
    }
}