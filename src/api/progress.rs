use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

use super::websocket::{ProgressWebSocket, WebSocketConfig};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ChannelType {
    Upload,
    Analysis,
    Batch,
    User,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ProgressEvent {
    Upload(UploadProgressEvent),
    Analysis(AnalysisProgressEvent),
    Batch(BatchProgressEvent),
    Error(ErrorEvent),
    Complete(CompleteEvent),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UploadProgressEvent {
    pub document_id: Uuid,
    pub file_name: String,
    pub bytes_uploaded: u64,
    pub total_bytes: u64,
    pub progress: u8,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalysisProgressEvent {
    pub analysis_id: Uuid,
    pub document_id: Uuid,
    pub stage: AnalysisStage,
    pub progress: u8,
    pub message: String,
    pub details: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AnalysisStage {
    Queued,
    Starting,
    ExtractingText,
    ChunkingText,
    GeneratingEmbeddings,
    AnalyzingContent,
    Finalizing,
    Completed,
}

impl AnalysisStage {
    pub fn display_name(&self) -> &str {
        match self {
            Self::Queued => "Queued",
            Self::Starting => "Starting",
            Self::ExtractingText => "Extracting Text",
            Self::ChunkingText => "Chunking Text",
            Self::GeneratingEmbeddings => "Generating Embeddings",
            Self::AnalyzingContent => "Analyzing Content",
            Self::Finalizing => "Finalizing",
            Self::Completed => "Completed",
        }
    }

    pub fn is_terminal(&self) -> bool {
        matches!(self, Self::Completed)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchProgressEvent {
    pub batch_id: Uuid,
    pub total_files: usize,
    pub completed_files: usize,
    pub current_file: Option<String>,
    pub overall_progress: u8,
    pub file_progress: HashMap<Uuid, FileProgress>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileProgress {
    pub document_id: Uuid,
    pub file_name: String,
    pub status: FileStatus,
    pub progress: u8,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FileStatus {
    Pending,
    Uploading,
    Processing,
    Completed,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorEvent {
    pub id: Uuid,
    pub error_type: ErrorType,
    pub message: String,
    pub details: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ErrorType {
    Upload,
    Analysis,
    System,
    Network,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompleteEvent {
    pub id: Uuid,
    pub event_type: CompleteEventType,
    pub message: String,
    pub result: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CompleteEventType {
    Upload,
    Analysis,
    Batch,
}

pub struct ProgressTracker {
    events: Arc<RwLock<HashMap<Uuid, Vec<ProgressEvent>>>>,
    websocket: Arc<RwLock<ProgressWebSocket>>,
    active_subscriptions: Arc<RwLock<HashMap<Uuid, ChannelType>>>,
}

impl ProgressTracker {
    pub fn new(config: WebSocketConfig, token: String) -> Self {
        let websocket = ProgressWebSocket::new(config, token);

        Self {
            events: Arc::new(RwLock::new(HashMap::new())),
            websocket: Arc::new(RwLock::new(websocket)),
            active_subscriptions: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn connect(&self) -> anyhow::Result<()> {
        let mut ws = self.websocket.write().await;
        ws.connect().await
    }

    pub async fn track_upload(&self, document_id: Uuid) -> anyhow::Result<()> {
        let mut ws = self.websocket.write().await;
        ws.subscribe_upload(document_id).await?;

        self.active_subscriptions
            .write()
            .await
            .insert(document_id, ChannelType::Upload);

        Ok(())
    }

    pub async fn track_analysis(&self, analysis_id: Uuid) -> anyhow::Result<()> {
        let mut ws = self.websocket.write().await;
        ws.subscribe_analysis(analysis_id).await?;

        self.active_subscriptions
            .write()
            .await
            .insert(analysis_id, ChannelType::Analysis);

        Ok(())
    }

    pub async fn track_batch(&self, batch_id: Uuid) -> anyhow::Result<()> {
        let mut ws = self.websocket.write().await;
        ws.subscribe_batch(batch_id).await?;

        self.active_subscriptions
            .write()
            .await
            .insert(batch_id, ChannelType::Batch);

        Ok(())
    }

    pub async fn get_events(&self, id: Uuid) -> Vec<ProgressEvent> {
        let events = self.events.read().await;
        events.get(&id).cloned().unwrap_or_default()
    }

    pub async fn get_latest_event(&self, id: Uuid) -> Option<ProgressEvent> {
        let events = self.events.read().await;
        events.get(&id).and_then(|e| e.last().cloned())
    }

    pub async fn process_events(&self) -> anyhow::Result<()> {
        let mut ws = self.websocket.write().await;

        while let Some(event) = ws.next_event().await {
            let id = match &event {
                ProgressEvent::Upload(e) => e.document_id,
                ProgressEvent::Analysis(e) => e.analysis_id,
                ProgressEvent::Batch(e) => e.batch_id,
                ProgressEvent::Error(e) => e.id,
                ProgressEvent::Complete(e) => e.id,
            };

            // Store event
            let mut events = self.events.write().await;
            events.entry(id).or_insert_with(Vec::new).push(event.clone());

            // Clean up completed subscriptions
            if matches!(&event, ProgressEvent::Complete(_) | ProgressEvent::Error(_)) {
                if let Some(channel_type) = self.active_subscriptions.read().await.get(&id) {
                    ws.unsubscribe(channel_type.clone(), id).await?;
                    self.active_subscriptions.write().await.remove(&id);
                }
            }
        }

        Ok(())
    }

    pub async fn start_processing(self: Arc<Self>) {
        tokio::spawn(async move {
            loop {
                if let Err(e) = self.process_events().await {
                    tracing::error!("Error processing events: {}", e);

                    // Try to reconnect
                    let mut ws = self.websocket.write().await;
                    if !ws.is_connected().await {
                        if let Err(e) = ws.handle_reconnect().await {
                            tracing::error!("Failed to reconnect: {}", e);
                            tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
                        }
                    }
                }
            }
        });
    }

    pub async fn disconnect(&self) {
        let mut ws = self.websocket.write().await;
        ws.disconnect().await;
    }
}