use anyhow::{Result, Context, bail};
use chrono::{DateTime, Utc};
use indicatif::{ProgressBar, ProgressStyle};
use reqwest::{Client, StatusCode, multipart};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize)]
pub struct UploadDocumentRequest {
    pub filename: String,
    pub category: Option<DocumentCategory>,
    pub description: Option<String>,
    pub tags: Option<Vec<String>>,
    pub mime_type: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DocumentCategory {
    Legal,
    Contract,
    Financial,
    Medical,
    Personal,
    Other,
}

#[derive(Debug, Deserialize)]
pub struct UploadDocumentResponse {
    pub document_id: Uuid,
    pub upload_url: String,
    pub upload_fields: serde_json::Value,
    pub expires_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
pub struct ConfirmUploadRequest {
    pub size_bytes: i64,
}

#[derive(Debug, Deserialize)]
pub struct DocumentResponse {
    pub id: Uuid,
    pub filename: String,
    pub category: Option<DocumentCategory>,
    pub size_bytes: Option<i64>,
    pub mime_type: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub download_url: Option<String>,
    pub analysis_status: Option<String>,
    pub analysis_id: Option<Uuid>,
    pub analyzed_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Deserialize)]
pub struct DocumentListResponse {
    pub documents: Vec<DocumentResponse>,
    pub total: i64,
    pub limit: i32,
    pub offset: i32,
}

#[derive(Debug, Deserialize)]
pub struct DocumentDownloadResponse {
    pub download_url: String,
    pub expires_at: DateTime<Utc>,
}

pub struct DocumentClient {
    client: Client,
    base_url: String,
}

impl DocumentClient {
    pub fn new(base_url: String) -> Self {
        let client = Client::builder()
            .timeout(std::time::Duration::from_secs(300)) // 5 minutes for large uploads
            .build()
            .expect("Failed to create HTTP client");

        Self { client, base_url }
    }

    /// Upload a document through the presigned URL flow
    pub async fn upload_document(
        &self,
        file_path: &Path,
        token: &str,
        category: Option<DocumentCategory>,
        description: Option<String>,
    ) -> Result<DocumentResponse> {
        // Read file metadata
        let metadata = fs::metadata(file_path)
            .context("Failed to read file metadata")?;
        let file_size = metadata.len() as i64;
        let filename = file_path.file_name()
            .and_then(|n| n.to_str())
            .ok_or_else(|| anyhow::anyhow!("Invalid filename"))?
            .to_string();

        // Determine MIME type
        let mime_type = match file_path.extension().and_then(|e| e.to_str()) {
            Some("pdf") => Some("application/pdf".to_string()),
            Some("doc") => Some("application/msword".to_string()),
            Some("docx") => Some("application/vnd.openxmlformats-officedocument.wordprocessingml.document".to_string()),
            Some("txt") => Some("text/plain".to_string()),
            _ => None,
        };

        println!("📤 Uploading: {}", filename);

        // Step 1: Request upload URL
        let upload_request = UploadDocumentRequest {
            filename: filename.clone(),
            category,
            description,
            tags: None,
            mime_type: mime_type.clone(),
        };

        let upload_response = self.request_upload_url(token, upload_request).await?;

        // Step 2: Upload file to presigned URL
        let file_content = fs::read(file_path)
            .context("Failed to read file content")?;

        self.upload_to_presigned_url(
            &upload_response.upload_url,
            &upload_response.upload_fields,
            file_content,
            &filename,
            mime_type.as_deref(),
        ).await?;

        // Step 3: Confirm upload
        let document = self.confirm_upload(
            token,
            upload_response.document_id,
            file_size,
        ).await?;

        println!("✅ Upload complete: {}", upload_response.document_id);
        Ok(document)
    }

    async fn request_upload_url(
        &self,
        token: &str,
        request: UploadDocumentRequest,
    ) -> Result<UploadDocumentResponse> {
        let url = format!("{}/documents", self.base_url);

        let response = self.client
            .post(&url)
            .header("Authorization", format!("Bearer {}", token))
            .json(&request)
            .send()
            .await
            .context("Failed to request upload URL")?;

        match response.status() {
            StatusCode::CREATED => {
                response.json::<UploadDocumentResponse>().await
                    .context("Failed to parse upload response")
            }
            StatusCode::UNAUTHORIZED => bail!("Authentication required"),
            StatusCode::FORBIDDEN => bail!("Insufficient permissions"),
            StatusCode::PAYLOAD_TOO_LARGE => bail!("File too large"),
            status => {
                let body = response.text().await.unwrap_or_default();
                bail!("Failed to request upload URL: {} - {}", status, body)
            }
        }
    }

    async fn upload_to_presigned_url(
        &self,
        upload_url: &str,
        upload_fields: &serde_json::Value,
        file_content: Vec<u8>,
        filename: &str,
        mime_type: Option<&str>,
    ) -> Result<()> {
        let pb = ProgressBar::new(file_content.len() as u64);
        pb.set_style(
            ProgressStyle::default_bar()
                .template("{spinner:.green} [{bar:40.cyan/blue}] {bytes}/{total_bytes} {msg}")
                .unwrap()
                .progress_chars("#>-")
        );
        pb.set_message("Uploading...");

        // Build multipart form
        let mut form = multipart::Form::new();

        // Add all fields from upload_fields
        if let Some(fields) = upload_fields.as_object() {
            for (key, value) in fields {
                if let Some(val) = value.as_str() {
                    form = form.text(key.clone(), val.to_string());
                }
            }
        }

        // Add the file as the last field (important for S3)
        let part = multipart::Part::bytes(file_content)
            .file_name(filename.to_string());

        let part = if let Some(mime) = mime_type {
            part.mime_str(mime)?
        } else {
            part
        };

        form = form.part("file", part);

        // Upload - Use a fresh client without auth headers for presigned URLs
        let upload_client = Client::builder()
            .timeout(std::time::Duration::from_secs(300))
            .build()?;

        let response = upload_client
            .post(upload_url)
            .multipart(form)
            .send()
            .await
            .context("Failed to upload file")?;

        pb.finish_with_message("Upload complete");

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            bail!("Failed to upload file: {} - {}", status, body)
        }

        Ok(())
    }

    async fn confirm_upload(
        &self,
        token: &str,
        document_id: Uuid,
        size_bytes: i64,
    ) -> Result<DocumentResponse> {
        let url = format!("{}/documents/{}/confirm", self.base_url, document_id);

        let response = self.client
            .post(&url)
            .header("Authorization", format!("Bearer {}", token))
            .json(&ConfirmUploadRequest { size_bytes })
            .send()
            .await
            .context("Failed to confirm upload")?;

        match response.status() {
            StatusCode::OK => {
                response.json::<DocumentResponse>().await
                    .context("Failed to parse document response")
            }
            StatusCode::NOT_FOUND => bail!("Document not found"),
            status => {
                let body = response.text().await.unwrap_or_default();
                bail!("Failed to confirm upload: {} - {}", status, body)
            }
        }
    }

    pub async fn get_document(
        &self,
        token: &str,
        document_id: Uuid,
    ) -> Result<DocumentResponse> {
        let url = format!("{}/documents/{}", self.base_url, document_id);

        let response = self.client
            .get(&url)
            .header("Authorization", format!("Bearer {}", token))
            .send()
            .await
            .context("Failed to get document")?;

        match response.status() {
            StatusCode::OK => {
                response.json::<DocumentResponse>().await
                    .context("Failed to parse document response")
            }
            StatusCode::NOT_FOUND => bail!("Document not found"),
            status => {
                let body = response.text().await.unwrap_or_default();
                bail!("Failed to get document: {} - {}", status, body)
            }
        }
    }

    /// List all documents for the authenticated user
    pub async fn list_documents(
        &self,
        token: &str,
        limit: Option<i32>,
        offset: Option<i32>,
    ) -> Result<DocumentListResponse> {
        let mut url = format!("{}/documents", self.base_url);

        let mut params = vec![];
        if let Some(limit) = limit {
            params.push(format!("limit={}", limit));
        }
        if let Some(offset) = offset {
            params.push(format!("offset={}", offset));
        }

        if !params.is_empty() {
            url = format!("{}?{}", url, params.join("&"));
        }

        let response = self.client
            .get(&url)
            .header("Authorization", format!("Bearer {}", token))
            .send()
            .await
            .context("Failed to list documents")?;

        match response.status() {
            StatusCode::OK => {
                response.json::<DocumentListResponse>().await
                    .context("Failed to parse document list response")
            }
            StatusCode::UNAUTHORIZED => bail!("Authentication required"),
            status => {
                let body = response.text().await.unwrap_or_default();
                bail!("Failed to list documents: {} - {}", status, body)
            }
        }
    }

    /// Delete a document
    pub async fn delete_document(
        &self,
        token: &str,
        document_id: Uuid,
    ) -> Result<()> {
        let url = format!("{}/documents/{}", self.base_url, document_id);

        let response = self.client
            .delete(&url)
            .header("Authorization", format!("Bearer {}", token))
            .send()
            .await
            .context("Failed to delete document")?;

        match response.status() {
            StatusCode::OK | StatusCode::NO_CONTENT => Ok(()),
            StatusCode::NOT_FOUND => bail!("Document not found"),
            StatusCode::FORBIDDEN => bail!("You don't have permission to delete this document"),
            status => {
                let body = response.text().await.unwrap_or_default();
                bail!("Failed to delete document: {} - {}", status, body)
            }
        }
    }

    /// Get download URL for a document
    pub async fn get_download_url(
        &self,
        token: &str,
        document_id: Uuid,
    ) -> Result<DocumentDownloadResponse> {
        let url = format!("{}/documents/{}/download", self.base_url, document_id);

        let response = self.client
            .get(&url)
            .header("Authorization", format!("Bearer {}", token))
            .send()
            .await
            .context("Failed to get download URL")?;

        match response.status() {
            StatusCode::OK => {
                response.json::<DocumentDownloadResponse>().await
                    .context("Failed to parse download response")
            }
            StatusCode::NOT_FOUND => bail!("Document not found"),
            StatusCode::FORBIDDEN => bail!("You don't have permission to download this document"),
            status => {
                let body = response.text().await.unwrap_or_default();
                bail!("Failed to get download URL: {} - {}", status, body)
            }
        }
    }

    /// Download a document to a local file
    pub async fn download_document(
        &self,
        token: &str,
        document_id: Uuid,
        output_path: Option<&Path>,
    ) -> Result<PathBuf> {
        // First get the document info to know the filename
        let document = self.get_document(token, document_id).await?;

        // Get the download URL
        let download_response = self.get_download_url(token, document_id).await?;

        // Download the file
        let pb = ProgressBar::new_spinner();
        pb.set_style(
            ProgressStyle::default_spinner()
                .template("{spinner:.cyan} Downloading {msg}")
                .unwrap()
        );
        pb.set_message(document.filename.clone());

        let response = self.client
            .get(&download_response.download_url)
            .send()
            .await
            .context("Failed to download file")?;

        if !response.status().is_success() {
            bail!("Failed to download file: {}", response.status());
        }

        // Determine output path
        let output_file = if let Some(path) = output_path {
            path.to_path_buf()
        } else {
            Path::new(&document.filename).to_path_buf()
        };

        // Save to file
        let bytes = response.bytes().await.context("Failed to read file content")?;
        fs::write(&output_file, bytes).context("Failed to write file")?;

        pb.finish_with_message(format!("Downloaded to {}", output_file.display()));

        Ok(output_file)
    }
}