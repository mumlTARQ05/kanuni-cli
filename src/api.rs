use anyhow::Result;
use reqwest::{Client, Response};
use serde::{Deserialize, Serialize};
use crate::config::Config;

pub struct ApiClient {
    client: Client,
    config: Config,
}

impl ApiClient {
    pub fn new(config: Config) -> Result<Self> {
        let client = Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()?;

        Ok(Self { client, config })
    }

    pub async fn analyze_document(&self, file_path: &str) -> Result<DocumentAnalysis> {
        // TODO: Implement actual API call
        Ok(DocumentAnalysis {
            document_type: "Contract".to_string(),
            parties: vec!["Party A".to_string(), "Party B".to_string()],
            key_dates: vec![],
            risks: vec![],
            summary: "Mock analysis result".to_string(),
        })
    }

    pub async fn chat(&self, message: &str, context: Option<&str>) -> Result<ChatResponse> {
        // TODO: Implement actual API call with streaming
        Ok(ChatResponse {
            message: "Mock response".to_string(),
            session_id: "mock-session".to_string(),
        })
    }

    pub async fn search_cases(&self, query: &str, filters: SearchFilters) -> Result<Vec<CaseResult>> {
        // TODO: Implement actual API call
        Ok(vec![])
    }

    fn get_headers(&self) -> reqwest::header::HeaderMap {
        let mut headers = reqwest::header::HeaderMap::new();

        if let Some(api_key) = &self.config.api_key {
            headers.insert(
                reqwest::header::AUTHORIZATION,
                format!("Bearer {}", api_key).parse().unwrap(),
            );
        }

        headers.insert(
            reqwest::header::USER_AGENT,
            format!("Kanuni/{}", env!("CARGO_PKG_VERSION")).parse().unwrap(),
        );

        headers
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DocumentAnalysis {
    pub document_type: String,
    pub parties: Vec<String>,
    pub key_dates: Vec<String>,
    pub risks: Vec<String>,
    pub summary: String,
}

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