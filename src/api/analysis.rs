use anyhow::{Result, Context, bail};
use chrono::{DateTime, Utc};
use indicatif::{ProgressBar, ProgressStyle};
use reqwest::{Client, StatusCode};
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tokio::time::sleep;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AnalysisType {
    Quick,
    Detailed,
    Legal,
    Financial,
    Medical,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AnalysisStatus {
    Pending,
    Processing,
    Completed,
    Failed,
    Cancelled,
}

#[derive(Debug, Serialize)]
pub struct StartAnalysisRequest {
    pub document_id: Uuid,
    pub analysis_type: AnalysisType,
    pub priority: Option<i32>,
    pub extract_entities: Option<bool>,
    pub extract_dates: Option<bool>,
    pub extract_financial: Option<bool>,
    pub perform_risk_assessment: Option<bool>,
}

#[derive(Debug, Deserialize)]
pub struct StartAnalysisResponse {
    pub analysis_id: Uuid,
    #[allow(dead_code)]
    pub document_id: Uuid,
    #[allow(dead_code)]
    pub analysis_type: AnalysisType,
    #[allow(dead_code)]
    pub status: AnalysisStatus,
    #[allow(dead_code)]
    pub created_at: DateTime<Utc>,
    #[allow(dead_code)]
    pub estimated_completion_time: Option<i32>, // seconds
}

#[derive(Debug, Deserialize)]
pub struct AnalysisStatusResponse {
    #[allow(dead_code)]
    pub id: Uuid,
    #[allow(dead_code)]
    pub document_id: Uuid,
    pub status: AnalysisStatus,
    pub progress: Option<i32>,
    #[allow(dead_code)]
    pub created_at: DateTime<Utc>,
    #[allow(dead_code)]
    pub started_at: Option<DateTime<Utc>>,
    #[allow(dead_code)]
    pub completed_at: Option<DateTime<Utc>>,
    pub error_message: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct AnalysisResultResponse {
    pub id: Uuid,
    #[allow(dead_code)]
    pub document_id: Uuid,
    pub analysis_type: AnalysisType,
    #[allow(dead_code)]
    pub status: AnalysisStatus,
    #[allow(dead_code)]
    pub result: Option<serde_json::Value>,
    pub summary: Option<String>,
    pub key_findings: Option<Vec<String>>,
    pub risk_assessment: Option<RiskAssessment>,
    pub entities: Option<Vec<Entity>>,
    pub dates: Option<Vec<ExtractedDate>>,
    #[allow(dead_code)]
    pub financial_data: Option<serde_json::Value>,
    #[allow(dead_code)]
    pub completed_at: DateTime<Utc>,
    pub processing_time_ms: Option<i64>,
}

#[derive(Debug, Deserialize)]
pub struct RiskAssessment {
    pub level: String, // Low, Medium, High
    pub factors: Vec<String>,
    pub recommendations: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub struct Entity {
    pub entity_type: String,
    pub value: String,
    pub confidence: f32,
}

#[derive(Debug, Deserialize)]
pub struct ExtractedDate {
    pub date: String,
    pub context: String,
    pub date_type: String, // deadline, effective_date, expiry_date, etc.
}

pub struct AnalysisClient {
    client: Client,
    base_url: String,
}

impl AnalysisClient {
    pub fn new(base_url: String) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(60))
            .build()
            .expect("Failed to create HTTP client");

        Self { client, base_url }
    }

    /// Start document analysis
    pub async fn start_analysis(
        &self,
        token: &str,
        document_id: Uuid,
        analysis_type: AnalysisType,
        options: AnalysisOptions,
    ) -> Result<StartAnalysisResponse> {
        let url = format!("{}/analysis/start", self.base_url);

        let request = StartAnalysisRequest {
            document_id,
            analysis_type,
            priority: options.priority,
            extract_entities: options.extract_entities,
            extract_dates: options.extract_dates,
            extract_financial: options.extract_financial,
            perform_risk_assessment: options.perform_risk_assessment,
        };

        let response = self.client
            .post(&url)
            .header("Authorization", format!("Bearer {}", token))
            .json(&request)
            .send()
            .await
            .context("Failed to start analysis")?;

        match response.status() {
            StatusCode::OK | StatusCode::CREATED => {
                response.json::<StartAnalysisResponse>().await
                    .context("Failed to parse analysis response")
            }
            StatusCode::UNAUTHORIZED => bail!("Authentication required"),
            StatusCode::FORBIDDEN => bail!("Insufficient permissions for this analysis type"),
            StatusCode::NOT_FOUND => bail!("Document not found"),
            StatusCode::TOO_MANY_REQUESTS => bail!("Rate limit exceeded. Please try again later."),
            status => {
                let body = response.text().await.unwrap_or_default();
                bail!("Failed to start analysis: {} - {}", status, body)
            }
        }
    }

    /// Get analysis status
    pub async fn get_status(
        &self,
        token: &str,
        analysis_id: Uuid,
    ) -> Result<AnalysisStatusResponse> {
        let url = format!("{}/analysis/{}/status", self.base_url, analysis_id);

        let response = self.client
            .get(&url)
            .header("Authorization", format!("Bearer {}", token))
            .send()
            .await
            .context("Failed to get analysis status")?;

        match response.status() {
            StatusCode::OK => {
                response.json::<AnalysisStatusResponse>().await
                    .context("Failed to parse status response")
            }
            StatusCode::NOT_FOUND => bail!("Analysis not found"),
            status => {
                let body = response.text().await.unwrap_or_default();
                bail!("Failed to get analysis status: {} - {}", status, body)
            }
        }
    }

    /// Get analysis results
    pub async fn get_result(
        &self,
        token: &str,
        analysis_id: Uuid,
    ) -> Result<AnalysisResultResponse> {
        let url = format!("{}/analysis/{}/result", self.base_url, analysis_id);

        let response = self.client
            .get(&url)
            .header("Authorization", format!("Bearer {}", token))
            .send()
            .await
            .context("Failed to get analysis result")?;

        match response.status() {
            StatusCode::OK => {
                response.json::<AnalysisResultResponse>().await
                    .context("Failed to parse result response")
            }
            StatusCode::NOT_FOUND => bail!("Analysis not found"),
            StatusCode::ACCEPTED => bail!("Analysis still in progress"),
            status => {
                let body = response.text().await.unwrap_or_default();
                bail!("Failed to get analysis result: {} - {}", status, body)
            }
        }
    }

    /// Wait for analysis to complete with progress updates
    pub async fn wait_for_completion(
        &self,
        token: &str,
        analysis_id: Uuid,
        timeout_secs: u64,
    ) -> Result<AnalysisResultResponse> {
        let pb = ProgressBar::new_spinner();
        pb.set_style(
            ProgressStyle::default_spinner()
                .template("{spinner:.cyan} {msg}")
                .unwrap()
        );

        let start_time = std::time::Instant::now();
        let timeout = Duration::from_secs(timeout_secs);

        loop {
            if start_time.elapsed() > timeout {
                pb.finish_with_message("❌ Analysis timed out");
                bail!("Analysis timed out after {} seconds", timeout_secs);
            }

            let status = self.get_status(token, analysis_id).await?;

            match status.status {
                AnalysisStatus::Completed => {
                    pb.finish_with_message("✅ Analysis complete");
                    return self.get_result(token, analysis_id).await;
                }
                AnalysisStatus::Failed => {
                    pb.finish_with_message("❌ Analysis failed");
                    bail!("Analysis failed: {}", status.error_message.unwrap_or_default());
                }
                AnalysisStatus::Cancelled => {
                    pb.finish_with_message("⚠️ Analysis cancelled");
                    bail!("Analysis was cancelled");
                }
                AnalysisStatus::Processing => {
                    let progress_msg = if let Some(progress) = status.progress {
                        format!("Processing... {}%", progress)
                    } else {
                        "Processing...".to_string()
                    };
                    pb.set_message(progress_msg);
                }
                AnalysisStatus::Pending => {
                    pb.set_message("Queued for processing...");
                }
            }

            sleep(Duration::from_secs(2)).await;
        }
    }

    /// Cancel an analysis
    #[allow(dead_code)]
    pub async fn cancel_analysis(
        &self,
        token: &str,
        analysis_id: Uuid,
    ) -> Result<()> {
        let url = format!("{}/analysis/{}/cancel", self.base_url, analysis_id);

        let response = self.client
            .delete(&url)
            .header("Authorization", format!("Bearer {}", token))
            .send()
            .await
            .context("Failed to cancel analysis")?;

        match response.status() {
            StatusCode::OK | StatusCode::NO_CONTENT => Ok(()),
            StatusCode::NOT_FOUND => bail!("Analysis not found"),
            status => {
                let body = response.text().await.unwrap_or_default();
                bail!("Failed to cancel analysis: {} - {}", status, body)
            }
        }
    }
}

#[derive(Debug, Default)]
pub struct AnalysisOptions {
    pub priority: Option<i32>,
    pub extract_entities: Option<bool>,
    pub extract_dates: Option<bool>,
    pub extract_financial: Option<bool>,
    pub perform_risk_assessment: Option<bool>,
}