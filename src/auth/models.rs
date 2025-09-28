use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoginRequest {
    pub email: String,
    pub password: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mfa_code: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct LoginResponse {
    pub user: UserInfo,
    pub access_token: String,
    pub refresh_token: String,
    pub expires_in: i64,
}

#[derive(Debug, Clone, Serialize)]
pub struct RefreshRequest {
    pub refresh_token: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RefreshResponse {
    pub user: UserInfo,
    pub access_token: String,
    pub expires_in: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserInfo {
    pub id: String,
    pub email: String,
    pub first_name: Option<String>,
    pub last_name: Option<String>,
    pub email_verified: bool,
    pub subscription_tier: Option<String>,
    pub mfa_enabled: bool,
}

#[derive(Debug, Clone)]
pub struct AuthTokens {
    pub access_token: String,
    #[allow(dead_code)]
    pub refresh_token: String,
    pub expires_at: DateTime<Utc>,
    pub user: UserInfo,
}

#[derive(Debug, Deserialize)]
pub struct ErrorResponse {
    #[allow(dead_code)]
    pub error: String,
    pub message: String,
}

// CLI Session Management Models

#[derive(Debug, Clone, Deserialize)]
pub struct CliSessionResponse {
    pub id: String,  // Using String for UUID compatibility
    pub device_name: Option<String>,
    pub platform: Option<String>,
    pub hostname: Option<String>,
    pub ip_address: Option<String>,
    pub last_used_at: DateTime<Utc>,
    pub scopes: Vec<String>,
    pub is_current: bool,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
pub struct RevokeSessionRequest {
    pub reason: Option<String>,
}
