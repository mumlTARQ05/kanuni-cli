pub mod analysis;
pub mod documents;

use crate::auth::AuthManager;
use crate::config::Config;
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::sync::Arc;
use uuid::Uuid;

pub use analysis::{AnalysisClient, AnalysisOptions, AnalysisResultResponse, AnalysisType};
pub use documents::{DocumentCategory, DocumentClient, DocumentListResponse, DocumentResponse};

pub struct ApiClient {
    #[allow(dead_code)]
    config: Arc<Config>,
    auth_manager: Arc<AuthManager>,
    document_client: DocumentClient,
    analysis_client: AnalysisClient,
}

impl ApiClient {
    pub fn new(config: Config) -> Result<Self> {
        let auth_manager = AuthManager::new(config.clone())?;
        let base_url = config.api_endpoint.clone();

        Ok(Self {
            config: Arc::new(config),
            auth_manager: Arc::new(auth_manager),
            document_client: DocumentClient::new(base_url.clone()),
            analysis_client: AnalysisClient::new(base_url),
        })
    }

    /// Upload and analyze a document in one flow
    pub async fn upload_and_analyze(
        &self,
        file_path: &Path,
        analysis_type: AnalysisType,
        category: Option<DocumentCategory>,
    ) -> Result<AnalysisResultResponse> {
        // Get auth token
        let token = self
            .auth_manager
            .get_access_token()
            .await
            .context("Authentication required. Please run 'kanuni auth login' first.")?;

        // Upload document
        println!("📤 Uploading document...");
        let document = self
            .document_client
            .upload_document(file_path, &token, category, None)
            .await?;

        // Start analysis
        println!(
            "🔍 Starting {} analysis...",
            format!("{:?}", analysis_type).to_lowercase()
        );
        let analysis_response = self
            .analysis_client
            .start_analysis(
                &token,
                document.id,
                analysis_type,
                AnalysisOptions::default(),
            )
            .await?;

        // Wait for completion
        println!("⏳ Waiting for analysis to complete...");
        let result = self
            .analysis_client
            .wait_for_completion(&token, analysis_response.analysis_id, 300) // 5 minute timeout
            .await?;

        Ok(result)
    }

    pub async fn analyze_existing_document(
        &self,
        document_id: Uuid,
        analysis_type: AnalysisType,
    ) -> Result<AnalysisResultResponse> {
        let token = self.auth_manager.get_access_token().await?;

        let analysis_response = self
            .analysis_client
            .start_analysis(
                &token,
                document_id,
                analysis_type,
                AnalysisOptions::default(),
            )
            .await?;

        let result = self
            .analysis_client
            .wait_for_completion(&token, analysis_response.analysis_id, 300)
            .await?;

        Ok(result)
    }

    #[allow(dead_code)]
    pub async fn chat(&self, _message: &str, _context: Option<&str>) -> Result<ChatResponse> {
        // TODO: Implement actual chat API call
        Ok(ChatResponse {
            message: "Chat functionality coming soon".to_string(),
            session_id: "mock-session".to_string(),
        })
    }

    #[allow(dead_code)]
    pub async fn search_cases(
        &self,
        _query: &str,
        _filters: SearchFilters,
    ) -> Result<Vec<CaseResult>> {
        // TODO: Implement actual search API call
        Ok(vec![])
    }

    /// List user documents
    pub async fn list_documents(
        &self,
        limit: Option<i32>,
        offset: Option<i32>,
    ) -> Result<DocumentListResponse> {
        let token = self
            .auth_manager
            .get_access_token()
            .await
            .context("Authentication required. Please run 'kanuni auth login' first.")?;

        self.document_client
            .list_documents(&token, limit, offset)
            .await
    }

    /// Get document details
    pub async fn get_document(&self, document_id: Uuid) -> Result<DocumentResponse> {
        let token = self
            .auth_manager
            .get_access_token()
            .await
            .context("Authentication required. Please run 'kanuni auth login' first.")?;

        self.document_client.get_document(&token, document_id).await
    }

    /// Delete a document
    pub async fn delete_document(&self, document_id: Uuid) -> Result<()> {
        let token = self
            .auth_manager
            .get_access_token()
            .await
            .context("Authentication required. Please run 'kanuni auth login' first.")?;

        self.document_client
            .delete_document(&token, document_id)
            .await
    }

    /// Download a document
    pub async fn download_document(
        &self,
        document_id: Uuid,
        output_path: Option<&Path>,
    ) -> Result<std::path::PathBuf> {
        let token = self
            .auth_manager
            .get_access_token()
            .await
            .context("Authentication required. Please run 'kanuni auth login' first.")?;

        self.document_client
            .download_document(&token, document_id, output_path)
            .await
    }

    /// Upload a document without analysis
    pub async fn upload_document(
        &self,
        file_path: &Path,
        category: Option<DocumentCategory>,
        description: Option<String>,
    ) -> Result<DocumentResponse> {
        let token = self
            .auth_manager
            .get_access_token()
            .await
            .context("Authentication required. Please run 'kanuni auth login' first.")?;

        self.document_client
            .upload_document(file_path, &token, category, description)
            .await
    }
}

// Legacy structs - will be replaced with proper implementations
#[derive(Debug, Serialize, Deserialize)]
pub struct ChatResponse {
    pub message: String,
    pub session_id: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SearchFilters {
    pub jurisdiction: Option<String>,
    pub date_range: Option<String>,
    pub limit: usize,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CaseResult {
    pub title: String,
    pub year: String,
    pub summary: String,
    pub relevance: f32,
}
