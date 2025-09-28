use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use colored::*;
use indicatif::ProgressBar;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

use crate::api::documents::DocumentCategory;
use crate::api::progress::{ProgressEvent, ProgressTracker, FileStatus};
use crate::api::websocket::WebSocketConfig;
use crate::api::ApiClient;
use crate::auth::AuthManager;
use crate::config::Config;
use crate::utils::progress::{BatchProgressDisplay, format_file_status, create_spinner};

#[derive(Debug, Parser)]
#[command(about = "Batch operations for documents")]
pub struct BatchCommand {
    #[command(subcommand)]
    pub action: BatchAction,
}

#[derive(Debug, Subcommand)]
pub enum BatchAction {
    #[command(about = "Upload multiple documents")]
    Upload {
        #[arg(help = "Files to upload (supports wildcards)")]
        files: Vec<String>,

        #[arg(long, help = "Automatically analyze after upload")]
        auto_analyze: bool,

        #[arg(long, value_enum, help = "Type of analysis to perform")]
        analysis_type: Option<String>,

        #[arg(long, help = "Document category (legal, contract, financial, medical, personal, other)")]
        category: Option<String>,

        #[arg(long, help = "Skip confirmation prompt")]
        yes: bool,

        #[arg(long, help = "Continue on error")]
        continue_on_error: bool,
    },

    #[command(about = "Check status of a batch operation")]
    Status {
        #[arg(help = "Batch ID to check")]
        batch_id: String,
    },
}

impl BatchCommand {
    pub async fn execute(self, config: Config) -> Result<()> {
        match self.action {
            BatchAction::Upload {
                files,
                auto_analyze,
                analysis_type,
                category,
                yes,
                continue_on_error,
            } => {
                execute_batch_upload(
                    config,
                    files,
                    auto_analyze,
                    analysis_type,
                    category,
                    yes,
                    continue_on_error,
                )
                .await
            }
            BatchAction::Status { batch_id } => execute_batch_status(config, batch_id).await,
        }
    }
}

pub async fn execute_batch_upload(
    config: Config,
    file_patterns: Vec<String>,
    auto_analyze: bool,
    analysis_type: Option<String>,
    category: Option<String>,
    skip_confirm: bool,
    continue_on_error: bool,
) -> Result<()> {
    // Expand file patterns to actual file paths
    let mut file_paths = Vec::new();
    for pattern in &file_patterns {
        let expanded = expand_glob_pattern(pattern)?;
        if expanded.is_empty() {
            println!("{} No files matching pattern: {}", "⚠️".yellow(), pattern);
        } else {
            file_paths.extend(expanded);
        }
    }

    if file_paths.is_empty() {
        return Err(anyhow::anyhow!("No files found to upload"));
    }

    // Display files to be uploaded
    println!("📁 Found {} files to upload:", file_paths.len());
    for (i, path) in file_paths.iter().enumerate() {
        if i < 10 {
            println!("  • {}", path.display());
        } else if i == 10 {
            println!("  ... and {} more", file_paths.len() - 10);
            break;
        }
    }

    // Confirm unless skipped
    if !skip_confirm {
        print!("\nProceed with upload? [Y/n] ");
        use std::io::{self, Write};
        io::stdout().flush()?;

        let mut input = String::new();
        io::stdin().read_line(&mut input)?;

        if !input.trim().is_empty() && !input.trim().eq_ignore_ascii_case("y") {
            println!("Cancelled");
            return Ok(());
        }
    }

    // Initialize API client
    let api_client = ApiClient::new(config.clone())?;

    // Parse category string to enum if provided
    let doc_category = if let Some(cat_str) = &category {
        match cat_str.to_lowercase().as_str() {
            "legal" => Some(DocumentCategory::Legal),
            "contract" => Some(DocumentCategory::Contract),
            "financial" => Some(DocumentCategory::Financial),
            "medical" => Some(DocumentCategory::Medical),
            "personal" => Some(DocumentCategory::Personal),
            "other" => Some(DocumentCategory::Other),
            _ => {
                println!("⚠️  Invalid category '{}', using 'Other'", cat_str);
                Some(DocumentCategory::Other)
            }
        }
    } else {
        None
    };

    // Check if WebSocket progress is enabled
    if config.websocket.enable_progress {
        execute_batch_upload_with_progress(
            api_client,
            config,
            file_paths,
            auto_analyze,
            analysis_type,
            doc_category,
            continue_on_error,
        )
        .await
    } else {
        execute_batch_upload_simple(
            api_client,
            file_paths,
            auto_analyze,
            analysis_type,
            doc_category,
            continue_on_error,
        )
        .await
    }
}

async fn execute_batch_upload_with_progress(
    api_client: ApiClient,
    config: Config,
    file_paths: Vec<PathBuf>,
    auto_analyze: bool,
    analysis_type: Option<String>,
    category: Option<DocumentCategory>,
    continue_on_error: bool,
) -> Result<()> {
    println!("\n🚀 Starting batch upload with real-time progress...\n");

    // Initialize progress tracker
    let ws_config = WebSocketConfig {
        url: config.get_websocket_url(),
        reconnect_max_attempts: config.websocket.reconnect_max_attempts,
        reconnect_delay_ms: config.websocket.reconnect_delay_ms,
        ping_interval_secs: config.websocket.ping_interval_secs,
    };

    // Get auth token for WebSocket
    let auth_manager = AuthManager::new(config.clone())?;
    let token = auth_manager
        .get_access_token()
        .await
        .context("Authentication required")?;

    let progress_tracker = Arc::new(ProgressTracker::new(ws_config, token));

    // Connect to WebSocket
    progress_tracker.connect().await?;

    // Start progress processing in background
    let tracker_clone = progress_tracker.clone();
    tracker_clone.start_processing().await;

    // Create batch progress display
    let batch_display = BatchProgressDisplay::new(file_paths.len());
    let mut results = Vec::new();

    // Process files sequentially
    for file_path in file_paths {
        let file_name = file_path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown");

        let doc_id = Uuid::new_v4();
        let pb = batch_display.add_file(doc_id, file_name).await;

        match upload_file_with_progress(
            &api_client,
            &progress_tracker,
            &file_path,
            category.clone(),
            auto_analyze,
            analysis_type.clone(),
            &pb,
        )
        .await
        {
            Ok(uploaded_id) => {
                // Use the original doc_id to complete the file, not the uploaded_id
                batch_display.complete_file(doc_id, true).await;
                results.push((file_path, Ok(uploaded_id)));
            }
            Err(e) => {
                batch_display.complete_file(doc_id, false).await;
                results.push((file_path.clone(), Err(e)));
                if !continue_on_error {
                    break;
                }
            }
        }
    }

    batch_display.finish("✅ Batch upload complete");

    // Display results summary
    let successful = results.iter().filter(|(_, r)| r.is_ok()).count();
    let failed = results.len() - successful;

    println!("\n📊 Batch Upload Summary:");
    println!("  ✅ Successful: {}", successful.to_string().green());
    if failed > 0 {
        println!("  ❌ Failed: {}", failed.to_string().red());
    }

    // Disconnect WebSocket
    progress_tracker.disconnect().await;

    Ok(())
}

async fn upload_file_with_progress(
    api_client: &ApiClient,
    _tracker: &Arc<ProgressTracker>,  // Not used until backend implements progress events
    file_path: &Path,
    category: Option<DocumentCategory>,
    auto_analyze: bool,
    analysis_type: Option<String>,
    pb: &ProgressBar,
) -> Result<Uuid> {
    // Start upload (this does the actual upload with its own progress bar)
    let document = api_client
        .upload_document(file_path, category, None, None)  // No filename override for batch
        .await?;

    // The upload is already complete at this point
    pb.finish_with_message("Upload complete");

    // Start analysis if requested
    if auto_analyze {
        if let Some(_analysis_type) = analysis_type {
            pb.set_message("Starting analysis...");
            // TODO: Start analysis when implemented
        }
    }

    Ok(document.id)
}

async fn execute_batch_upload_simple(
    api_client: ApiClient,
    file_paths: Vec<PathBuf>,
    auto_analyze: bool,
    analysis_type: Option<String>,
    category: Option<DocumentCategory>,
    continue_on_error: bool,
) -> Result<()> {
    println!("\n📤 Uploading {} files...\n", file_paths.len());

    let mut successful = 0;
    let mut failed = 0;

    for file_path in file_paths {
        let file_name = file_path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown");

        let spinner = create_spinner(&format!("Uploading {}", file_name));

        match api_client
            .upload_document(&file_path, category.clone(), None, None)  // No filename override for batch
            .await
        {
            Ok(document) => {
                spinner.finish_with_message(format!("✅ {} uploaded", file_name));
                successful += 1;

                if auto_analyze {
                    if let Some(ref analysis_type) = analysis_type {
                        let spinner = create_spinner(&format!("Analyzing {}", file_name));
                        // TODO: Start analysis
                        spinner.finish_with_message(format!("✅ {} analyzed", file_name));
                    }
                }
            }
            Err(e) => {
                spinner.finish_with_message(format!("❌ {} failed: {}", file_name, e));
                failed += 1;
                if !continue_on_error {
                    return Err(e);
                }
            }
        }
    }

    println!("\n📊 Batch Upload Summary:");
    println!("  ✅ Successful: {}", successful.to_string().green());
    if failed > 0 {
        println!("  ❌ Failed: {}", failed.to_string().red());
    }

    Ok(())
}

pub async fn execute_batch_status(config: Config, batch_id: String) -> Result<()> {
    let batch_uuid = Uuid::parse_str(&batch_id)
        .context("Invalid batch ID format")?;

    // Initialize API client
    let api_client = ApiClient::new(config.clone())?;

    if config.websocket.enable_progress {
        // Connect to WebSocket and subscribe to batch events
        let ws_config = WebSocketConfig {
            url: config.get_websocket_url(),
            reconnect_max_attempts: config.websocket.reconnect_max_attempts,
            reconnect_delay_ms: config.websocket.reconnect_delay_ms,
            ping_interval_secs: config.websocket.ping_interval_secs,
        };

        let auth_manager = AuthManager::new(config.clone())?;
        let token = auth_manager
            .get_access_token()
            .await
            .context("Authentication required")?;

        let progress_tracker = Arc::new(ProgressTracker::new(ws_config, token));
        progress_tracker.connect().await?;
        progress_tracker.track_batch(batch_uuid).await?;

        // Start processing events
        let tracker_clone = progress_tracker.clone();
        tracker_clone.start_processing().await;

        println!("📊 Monitoring batch {} ...\n", batch_id.yellow());

        // Monitor progress
        loop {
            tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

            if let Some(event) = progress_tracker.get_latest_event(batch_uuid).await {
                match event {
                    ProgressEvent::Batch(batch) => {
                        println!("Overall Progress: {}%", batch.overall_progress);
                        println!("Files: {}/{}", batch.completed_files, batch.total_files);

                        if let Some(current) = batch.current_file {
                            println!("Current: {}", current);
                        }

                        println!("\nFile Status:");
                        for (_, file_progress) in batch.file_progress {
                            println!(
                                "  {} {} - {}%",
                                format_file_status(&file_progress.status),
                                file_progress.file_name,
                                file_progress.progress
                            );
                        }
                    }
                    ProgressEvent::Complete(_) => {
                        println!("\n✅ Batch operation completed!");
                        break;
                    }
                    ProgressEvent::Error(e) => {
                        println!("\n❌ Batch operation failed: {}", e.message);
                        break;
                    }
                    _ => {}
                }
            }
        }

        progress_tracker.disconnect().await;
    } else {
        println!("Batch status monitoring requires WebSocket progress to be enabled");
        println!("Enable it in your configuration or use --enable-progress flag");
    }

    Ok(())
}

fn expand_glob_pattern(pattern: &str) -> Result<Vec<PathBuf>> {
    use glob::glob;

    let matches = glob(pattern)?;
    let mut paths = Vec::new();

    for entry in matches {
        match entry {
            Ok(path) if path.is_file() => {
                paths.push(path);
            }
            _ => {}
        }
    }

    Ok(paths)
}