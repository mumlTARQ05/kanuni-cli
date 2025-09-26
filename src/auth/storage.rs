use anyhow::{Result, Context};
use keyring::Entry;

const SERVICE_NAME: &str = "kanuni";
const REFRESH_TOKEN_KEY: &str = "refresh_token";

pub struct TokenStorage {
    service: String,
}

impl TokenStorage {
    pub fn new() -> Result<Self> {
        Ok(Self {
            service: SERVICE_NAME.to_string(),
        })
    }

    pub fn store_refresh_token(&self, token: &str) -> Result<()> {
        let entry = Entry::new(&self.service, REFRESH_TOKEN_KEY)
            .context("Failed to create keyring entry")?;

        entry.set_password(token)
            .context("Failed to store refresh token in keyring")?;

        Ok(())
    }

    pub fn get_refresh_token(&self) -> Result<String> {
        let entry = Entry::new(&self.service, REFRESH_TOKEN_KEY)
            .context("Failed to create keyring entry")?;

        entry.get_password()
            .context("Failed to retrieve refresh token from keyring")
    }

    pub fn clear_tokens(&self) -> Result<()> {
        let entry = Entry::new(&self.service, REFRESH_TOKEN_KEY)
            .context("Failed to create keyring entry")?;

        // Try to delete, but don't fail if it doesn't exist
        let _ = entry.delete_credential();

        Ok(())
    }
}