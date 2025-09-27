use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoredCredentials {
    pub auth_type: AuthType,
    pub user_id: Option<Uuid>,
    pub email: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AuthType {
    OAuth {
        access_token: String,
        refresh_token: String,
        expires_at: DateTime<Utc>,
    },
    ApiKey {
        key: String,
        name: String,
        prefix: String,
        last_4: String,
    },
}

pub struct TokenStore {
    config_dir: PathBuf,
}

impl TokenStore {
    pub fn new() -> Result<Self> {
        let config_dir = directories::ProjectDirs::from("ai", "v-lawyer", "kanuni")
            .context("Failed to get config directory")?
            .config_dir()
            .to_path_buf();

        // Create config directory if it doesn't exist
        fs::create_dir_all(&config_dir)?;

        Ok(Self { config_dir })
    }

    fn auth_file_path(&self) -> PathBuf {
        self.config_dir.join("auth.json")
    }

    pub fn save_credentials(&self, credentials: StoredCredentials) -> Result<()> {
        let auth_file = self.auth_file_path();

        // Serialize credentials
        let json = serde_json::to_string_pretty(&credentials)?;

        // Write to file with restricted permissions
        fs::write(&auth_file, json)?;

        // Set file permissions to 600 (owner read/write only) on Unix
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = fs::metadata(&auth_file)?.permissions();
            perms.set_mode(0o600);
            fs::set_permissions(&auth_file, perms)?;
        }

        Ok(())
    }

    pub fn load_credentials(&self) -> Result<Option<StoredCredentials>> {
        let auth_file = self.auth_file_path();

        if !auth_file.exists() {
            return Ok(None);
        }

        let contents = fs::read_to_string(&auth_file).context("Failed to read auth file")?;

        let credentials: StoredCredentials =
            serde_json::from_str(&contents).context("Failed to parse auth file")?;

        Ok(Some(credentials))
    }

    pub fn clear_credentials(&self) -> Result<()> {
        let auth_file = self.auth_file_path();

        if auth_file.exists() {
            fs::remove_file(&auth_file)?;
        }

        Ok(())
    }

    pub fn update_oauth_tokens(
        &self,
        access_token: String,
        refresh_token: String,
        expires_at: DateTime<Utc>,
    ) -> Result<()> {
        let mut credentials = self
            .load_credentials()?
            .context("No credentials to update")?;

        match &mut credentials.auth_type {
            AuthType::OAuth {
                access_token: at,
                refresh_token: rt,
                expires_at: exp,
            } => {
                *at = access_token;
                *rt = refresh_token;
                *exp = expires_at;
                credentials.updated_at = Utc::now();
                self.save_credentials(credentials)?;
                Ok(())
            }
            _ => anyhow::bail!("Cannot update OAuth tokens for API key auth"),
        }
    }
}
