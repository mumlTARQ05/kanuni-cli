use console::Term;
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

use crate::api::progress::{AnalysisStage, FileStatus};

/// Create a progress bar for document upload
pub fn create_upload_progress_bar(file_name: &str, total_bytes: u64) -> ProgressBar {
    let pb = ProgressBar::new(total_bytes);
    pb.set_style(
        ProgressStyle::default_bar()
            .template("{spinner:.green} Uploading {prefix:.bold} [{bar:40.cyan/blue}] {bytes}/{total_bytes} ({percent}%) | {msg}")
            .unwrap()
            .progress_chars("#>-"),
    );
    pb.set_prefix(file_name.to_string());
    pb
}

/// Create a progress bar for analysis
pub fn create_analysis_progress_bar() -> ProgressBar {
    let pb = ProgressBar::new(100);
    pb.set_style(
        ProgressStyle::default_bar()
            .template("{spinner:.green} {prefix:.bold.yellow} [{bar:40.cyan/blue}] {percent}% | {msg}")
            .unwrap()
            .progress_chars("#>-"),
    );
    pb.set_prefix("Analyzing");
    pb
}

/// Format analysis stage for display
pub fn format_stage(stage: &AnalysisStage) -> String {
    match stage {
        AnalysisStage::Queued => "⏳ Queued",
        AnalysisStage::Starting => "🚀 Starting",
        AnalysisStage::ExtractingText => "📄 Extracting Text",
        AnalysisStage::ChunkingText => "✂️ Chunking Text",
        AnalysisStage::GeneratingEmbeddings => "🧮 Generating Embeddings",
        AnalysisStage::AnalyzingContent => "🔍 Analyzing Content",
        AnalysisStage::Finalizing => "📝 Finalizing",
        AnalysisStage::Completed => "✅ Completed",
    }
    .to_string()
}

/// Multi-progress bar manager for batch operations
pub struct BatchProgressDisplay {
    multi_bar: MultiProgress,
    bars: Arc<RwLock<HashMap<Uuid, ProgressBar>>>,
    overall_bar: ProgressBar,
}

impl BatchProgressDisplay {
    pub fn new(total_files: usize) -> Self {
        let multi_bar = MultiProgress::new();

        // Overall progress bar
        let overall_bar = multi_bar.add(ProgressBar::new(total_files as u64));
        overall_bar.set_style(
            ProgressStyle::default_bar()
                .template("📦 Overall Progress [{bar:50.cyan/blue}] {pos}/{len} files ({percent}%) | {msg}")
                .unwrap()
                .progress_chars("#>-"),
        );

        Self {
            multi_bar,
            bars: Arc::new(RwLock::new(HashMap::new())),
            overall_bar,
        }
    }

    pub async fn add_file(&self, document_id: Uuid, file_name: &str) -> ProgressBar {
        let pb = self.multi_bar.add(ProgressBar::new(100));
        pb.set_style(
            ProgressStyle::default_bar()
                .template("  {prefix:.bold} [{bar:30.green/red}] {percent}% | {msg}")
                .unwrap()
                .progress_chars("#>-"),
        );
        pb.set_prefix(format!("📄 {}", file_name));

        self.bars.write().await.insert(document_id, pb.clone());
        pb
    }

    pub async fn update_file(&self, document_id: Uuid, progress: u8, message: String) {
        if let Some(pb) = self.bars.read().await.get(&document_id) {
            pb.set_position(progress as u64);
            pb.set_message(message);
        }
    }

    pub async fn complete_file(&self, document_id: Uuid, success: bool) {
        if let Some(pb) = self.bars.read().await.get(&document_id) {
            if success {
                pb.finish_with_message("✅ Complete");
            } else {
                pb.finish_with_message("❌ Failed");
            }
        }
        self.overall_bar.inc(1);
    }

    pub fn finish(&self, message: &str) {
        self.overall_bar.finish_with_message(message.to_string());
    }
}

/// Live status display for real-time updates
pub struct LiveStatusDisplay {
    term: Term,
    lines: Arc<RwLock<Vec<String>>>,
    max_lines: usize,
}

impl LiveStatusDisplay {
    pub fn new(max_lines: usize) -> Self {
        Self {
            term: Term::stdout(),
            lines: Arc::new(RwLock::new(Vec::new())),
            max_lines,
        }
    }

    pub async fn add_status(&self, icon: &str, message: String) {
        let mut lines = self.lines.write().await;
        let formatted = format!("{} {}", icon, message);
        lines.push(formatted);

        // Keep only the last N lines
        if lines.len() > self.max_lines {
            lines.remove(0);
        }

        self.render().await;
    }

    pub async fn update_last(&self, icon: &str, message: String) {
        let mut lines = self.lines.write().await;
        let formatted = format!("{} {}", icon, message);

        if lines.is_empty() {
            lines.push(formatted);
        } else {
            let idx = lines.len() - 1;
            lines[idx] = formatted;
        }

        self.render().await;
    }

    async fn render(&self) {
        let _ = self.term.clear_last_lines(self.max_lines);
        let lines = self.lines.read().await;
        for line in lines.iter() {
            let _ = self.term.write_line(line);
        }
    }

    pub async fn clear(&self) {
        let _ = self.term.clear_last_lines(self.max_lines);
        self.lines.write().await.clear();
    }
}

/// Format file status with appropriate icon
pub fn format_file_status(status: &FileStatus) -> String {
    match status {
        FileStatus::Pending => "⏳ Pending",
        FileStatus::Uploading => "📤 Uploading",
        FileStatus::Processing => "⚙️ Processing",
        FileStatus::Completed => "✅ Completed",
        FileStatus::Failed => "❌ Failed",
    }
    .to_string()
}

/// Create a spinner for indeterminate operations
pub fn create_spinner(message: &str) -> ProgressBar {
    let pb = ProgressBar::new_spinner();
    pb.set_style(
        ProgressStyle::default_spinner()
            .template("{spinner:.green} {msg}")
            .unwrap(),
    );
    pb.set_message(message.to_string());
    pb.enable_steady_tick(std::time::Duration::from_millis(100));
    pb
}

/// Format bytes into human-readable string
pub fn format_bytes(bytes: u64) -> String {
    const UNITS: &[&str] = &["B", "KB", "MB", "GB", "TB"];
    let mut size = bytes as f64;
    let mut unit_index = 0;

    while size >= 1024.0 && unit_index < UNITS.len() - 1 {
        size /= 1024.0;
        unit_index += 1;
    }

    if unit_index == 0 {
        format!("{} {}", size as u64, UNITS[unit_index])
    } else {
        format!("{:.2} {}", size, UNITS[unit_index])
    }
}

/// Calculate ETA based on progress and elapsed time
pub fn calculate_eta(progress: f64, elapsed_secs: f64) -> String {
    if progress == 0.0 {
        return "calculating...".to_string();
    }

    let total_time = elapsed_secs / progress;
    let remaining_secs = total_time - elapsed_secs;

    if remaining_secs < 60.0 {
        format!("{:.0}s", remaining_secs)
    } else if remaining_secs < 3600.0 {
        format!("{:.0}m {:.0}s", remaining_secs / 60.0, remaining_secs % 60.0)
    } else {
        format!(
            "{:.0}h {:.0}m",
            remaining_secs / 3600.0,
            (remaining_secs % 3600.0) / 60.0
        )
    }
}