use anyhow::Result;
use reqwest::{Client, StatusCode};
use colored::*;
use chrono::{DateTime, Utc};

use super::models::{CliSessionResponse, RevokeSessionRequest, ErrorResponse};

pub struct SessionsClient {
    client: Client,
    base_url: String,
}

impl SessionsClient {
    pub fn new(base_url: String) -> Self {
        let client = Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .expect("Failed to create HTTP client");

        Self { client, base_url }
    }

    /// List all CLI sessions
    pub async fn list_sessions(&self, access_token: &str) -> Result<Vec<CliSessionResponse>> {
        let url = format!("{}/auth/cli/sessions", self.base_url);

        let response = self
            .client
            .get(&url)
            .header("Authorization", format!("Bearer {}", access_token))
            .send()
            .await?;

        match response.status() {
            StatusCode::OK => {
                let sessions = response
                    .json::<Vec<CliSessionResponse>>()
                    .await?;
                Ok(sessions)
            }
            StatusCode::UNAUTHORIZED => {
                anyhow::bail!("Authentication token expired. Please login again.")
            }
            status => {
                if let Ok(error) = response.json::<ErrorResponse>().await {
                    anyhow::bail!("{}: {}", status, error.message)
                } else {
                    anyhow::bail!("Failed to list sessions with status: {}", status)
                }
            }
        }
    }

    /// Revoke a specific CLI session
    pub async fn revoke_session(&self, access_token: &str, session_id: &str) -> Result<()> {
        let url = format!("{}/auth/cli/sessions/{}", self.base_url, session_id);

        let request = RevokeSessionRequest {
            reason: Some("User revoked from CLI".to_string()),
        };

        let response = self
            .client
            .delete(&url)
            .header("Authorization", format!("Bearer {}", access_token))
            .json(&request)
            .send()
            .await?;

        match response.status() {
            StatusCode::OK => Ok(()),
            StatusCode::NOT_FOUND => {
                anyhow::bail!("Session not found or already revoked")
            }
            StatusCode::UNAUTHORIZED => {
                anyhow::bail!("Authentication token expired. Please login again.")
            }
            status => {
                if let Ok(error) = response.json::<ErrorResponse>().await {
                    anyhow::bail!("{}: {}", status, error.message)
                } else {
                    anyhow::bail!("Failed to revoke session with status: {}", status)
                }
            }
        }
    }

    /// Revoke all CLI sessions
    pub async fn revoke_all_sessions(&self, access_token: &str) -> Result<()> {
        let url = format!("{}/auth/cli/sessions/revoke-all", self.base_url);

        let response = self
            .client
            .post(&url)
            .header("Authorization", format!("Bearer {}", access_token))
            .send()
            .await?;

        match response.status() {
            StatusCode::OK => Ok(()),
            StatusCode::UNAUTHORIZED => {
                anyhow::bail!("Authentication token expired. Please login again.")
            }
            status => {
                if let Ok(error) = response.json::<ErrorResponse>().await {
                    anyhow::bail!("{}: {}", status, error.message)
                } else {
                    anyhow::bail!("Failed to revoke all sessions with status: {}", status)
                }
            }
        }
    }
}

// Helper functions for formatting sessions

pub fn format_session_display(sessions: &[CliSessionResponse]) {
    if sessions.is_empty() {
        println!("{}  No active CLI sessions found", "ℹ".blue());
        return;
    }

    println!("\n{}  Active CLI Sessions\n", "🔐".green());

    // Header
    println!(
        "{:<12} {:<20} {:<15} {:<20} {:<15} {}",
        "ID".bold(),
        "Device".bold(),
        "Platform".bold(),
        "Hostname".bold(),
        "Last Active".bold(),
        "Status".bold()
    );

    println!("{}", "─".repeat(100));

    for session in sessions {
        let id_short = &session.id[..8.min(session.id.len())];
        let device_name = session.device_name.as_deref().unwrap_or("Unknown");
        let platform = format_platform(session.platform.as_deref());
        let hostname = session.hostname.as_deref().unwrap_or("-");
        let last_active = format_relative_time(&session.last_used_at);

        let status = if session.is_current {
            "CURRENT".green().bold()
        } else if session.is_active {
            "Active".green()
        } else {
            "Inactive".red()
        };

        println!(
            "{:<12} {:<20} {:<15} {:<20} {:<15} {}",
            id_short,
            truncate_string(device_name, 20),
            platform,
            truncate_string(hostname, 20),
            last_active,
            status
        );
    }

    println!("\n{}  Total: {} active session(s)", "📊".blue(), sessions.len());
}

fn format_platform(platform: Option<&str>) -> String {
    match platform {
        Some(p) if p.to_lowercase().contains("darwin") => "macOS",
        Some(p) if p.to_lowercase().contains("linux") => "Linux",
        Some(p) if p.to_lowercase().contains("win") => "Windows",
        Some(p) if p == "cli" => "CLI",
        Some(p) => p,
        None => "Unknown",
    }.to_string()
}

fn truncate_string(s: &str, max_len: usize) -> String {
    if s.len() > max_len {
        format!("{}...", &s[..max_len - 3])
    } else {
        s.to_string()
    }
}

fn format_relative_time(dt: &DateTime<Utc>) -> String {
    let now = Utc::now();
    let duration = now.signed_duration_since(*dt);

    if duration.num_days() > 30 {
        format!("{} months ago", duration.num_days() / 30)
    } else if duration.num_days() > 0 {
        format!("{} days ago", duration.num_days())
    } else if duration.num_hours() > 0 {
        format!("{} hours ago", duration.num_hours())
    } else if duration.num_minutes() > 0 {
        format!("{} mins ago", duration.num_minutes())
    } else {
        "Just now".to_string()
    }
}