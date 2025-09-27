use anyhow::{Result, bail};
use chrono_humanize::HumanTime;
use colored::*;
use comfy_table::{Table, presets::UTF8_FULL, ContentArrangement, Cell, Color as TableColor};
use std::path::Path;
use uuid::Uuid;

use crate::api::{ApiClient, documents::AnalysisStatus};
use crate::config::Config;
use crate::cli::DocumentAction;

pub async fn execute(action: &DocumentAction) -> Result<()> {
    let config = Config::load()?;
    let api_client = ApiClient::new(config)?;

    match action {
        DocumentAction::Upload { file, category, description } => {
            upload_document(&api_client, file, category.as_deref(), description.as_deref()).await
        },
        DocumentAction::List { limit, offset } => list_documents(&api_client, *limit, *offset).await,
        DocumentAction::Info { id } => show_document_info(&api_client, id).await,
        DocumentAction::Delete { id } => delete_document(&api_client, id).await,
        DocumentAction::Download { id, output } => download_document(&api_client, id, output.as_deref()).await,
    }
}

async fn upload_document(
    api_client: &ApiClient,
    file_path: &str,
    category: Option<&str>,
    description: Option<&str>,
) -> Result<()> {
    use std::path::Path;
    use crate::api::DocumentCategory;

    println!("{} {}", "📤".cyan(), format!("Uploading document: {}", file_path).bold());

    // Check if file exists
    let path = Path::new(file_path);
    if !path.exists() {
        bail!("File not found: {}", file_path);
    }

    // Parse category if provided
    let category_enum = if let Some(cat) = category {
        match cat.to_lowercase().as_str() {
            "legal" => Some(DocumentCategory::Legal),
            "contract" => Some(DocumentCategory::Contract),
            "financial" => Some(DocumentCategory::Financial),
            "medical" => Some(DocumentCategory::Medical),
            "personal" => Some(DocumentCategory::Personal),
            "other" => Some(DocumentCategory::Other),
            _ => {
                println!("{}", "Warning: Invalid category. Using 'Other'.".yellow());
                Some(DocumentCategory::Other)
            }
        }
    } else {
        None
    };

    // Upload document
    let document = api_client.upload_document(path, category_enum, description.map(|s| s.to_string())).await?;

    println!("\n{} {}", "✅".green(), "Document uploaded successfully!".bold());
    println!("  {} {}", "ID:".bright_black(), document.id.to_string().yellow());
    println!("  {} {}", "Filename:".bright_black(), document.filename.white());
    println!("  {} {:?}", "Category:".bright_black(), document.category);
    println!("  {} {}", "Size:".bright_black(), format_file_size(document.size_bytes));
    println!("\n{}", "To analyze this document, run:".bright_black());
    println!("  {} {}", "➜".cyan(), format!("kanuni analyze --document-id {}", document.id).yellow());

    Ok(())
}

async fn list_documents(api_client: &ApiClient, limit: Option<i32>, offset: Option<i32>) -> Result<()> {
    println!("{} Fetching your documents...", "📄".cyan());

    let response = api_client.list_documents(limit, offset).await?;

    if response.documents.is_empty() {
        println!("\n{}", "No documents found. Upload documents using 'kanuni analyze <file>'".yellow());
        return Ok(());
    }

    println!("\n{} {}", "📄".cyan(), format!("Your Documents ({} total):", response.total).bold());

    // Create a table for document list
    let mut table = Table::new();
    table
        .load_preset(UTF8_FULL)
        .set_content_arrangement(ContentArrangement::Dynamic)
        .set_header(vec!["ID", "Filename", "Category", "Size", "Uploaded", "Status"]);

    for doc in &response.documents {
        // Format document ID (show first 8 chars)
        let short_id = doc.id.to_string()[..8].to_string();

        // Format file size
        let size = format_file_size(doc.size_bytes);

        // Format upload time as human-readable
        let uploaded = HumanTime::from(doc.upload_date).to_string();

        // Format analysis status
        let status = match doc.analysis_status {
            AnalysisStatus::Completed => "✅".to_string(),
            AnalysisStatus::Processing | AnalysisStatus::Analyzing => "⏳".to_string(),
            AnalysisStatus::Failed => "❌".to_string(),
            AnalysisStatus::Pending => "🔄".to_string(),
        };

        // Format category
        let category = format!("{:?}", doc.category);

        table.add_row(vec![
            Cell::new(&short_id).fg(TableColor::Yellow),
            Cell::new(&doc.filename),
            Cell::new(&category),
            Cell::new(&size),
            Cell::new(&uploaded),
            Cell::new(&status),
        ]);
    }

    println!("{}", table);

    if response.documents.len() < response.total as usize {
        println!("\n{}", format!("Showing {} of {} documents. Use --limit and --offset to see more.",
            response.documents.len(), response.total).bright_black());
    }

    Ok(())
}

async fn show_document_info(api_client: &ApiClient, id: &str) -> Result<()> {
    // Parse UUID
    let document_id = parse_document_id(id)?;

    println!("{} Fetching document details...", "📄".cyan());

    let document = api_client.get_document(document_id).await?;

    // Analysis details are included in the document response

    println!("\n{} {}", "📄".cyan(), "Document Details:".bold());
    println!("  {} {}", "ID:".bright_black(), document.id.to_string().yellow());
    println!("  {} {}", "Filename:".bright_black(), document.filename.white());
    println!("  {} {:?}", "Category:".bright_black(), document.category);
    println!("  {} {}", "Size:".bright_black(), format_file_size(document.size_bytes));
    println!("  {} {}", "Type:".bright_black(), document.mime_type);
    println!("  {} {}", "Uploaded:".bright_black(), document.upload_date.format("%Y-%m-%d %H:%M:%S UTC"));

    // Analysis status
    let status_display = match document.analysis_status {
        AnalysisStatus::Completed => "Completed ✅".green(),
        AnalysisStatus::Processing | AnalysisStatus::Analyzing => "Processing ⏳".yellow(),
        AnalysisStatus::Failed => "Failed ❌".red(),
        AnalysisStatus::Pending => "Pending 🔄".blue(),
    };

    println!("\n{} {}", "📊".cyan(), "Analysis Status:".bold());
    println!("  {}", status_display);

    if let Some(analysis_id) = document.analysis_id {
        println!("  {} {}", "Analysis ID:".bright_black(), analysis_id.to_string().yellow());
    }

    if document.analysis_status == AnalysisStatus::Pending {
        println!("  {}", "Run 'kanuni analyze' with this document to start analysis".bright_black());
    }

    // Download URL
    if document.download_url.is_some() {
        println!("\n{}", format!("💾 Download available. Use 'kanuni document download {}' to download", id).bright_black());
    }

    Ok(())
}

async fn delete_document(api_client: &ApiClient, id: &str) -> Result<()> {
    let document_id = parse_document_id(id)?;

    // Get document info first
    let document = api_client.get_document(document_id).await?;

    println!("{} {}", "🗑️".red(), format!("Deleting '{}'...", document.filename).bold());

    // Confirm deletion
    print!("Are you sure you want to delete this document? (y/N): ");
    use std::io::{self, Write};
    io::stdout().flush()?;

    let mut input = String::new();
    io::stdin().read_line(&mut input)?;

    if !input.trim().eq_ignore_ascii_case("y") {
        println!("{}", "Deletion cancelled.".yellow());
        return Ok(());
    }

    api_client.delete_document(document_id).await?;

    println!("{} {}", "✅".green(), format!("Document '{}' deleted successfully", document.filename).bold());

    Ok(())
}

async fn download_document(api_client: &ApiClient, id: &str, output: Option<&str>) -> Result<()> {
    let document_id = parse_document_id(id)?;

    let output_path = output.map(Path::new);

    let downloaded_path = api_client.download_document(document_id, output_path).await?;

    println!("{} {}", "✅".green(), format!("Document downloaded to: {}", downloaded_path.display()).bold());

    Ok(())
}

// Helper functions

fn parse_document_id(id: &str) -> Result<Uuid> {
    // Try to parse as full UUID
    if let Ok(uuid) = Uuid::parse_str(id) {
        return Ok(uuid);
    }

    // If it's a short ID (8 chars), we can't expand it without a lookup
    // For now, require full UUIDs
    bail!("Invalid document ID. Please provide the full UUID.")
}

fn format_file_size(bytes: i64) -> String {
    const UNITS: &[&str] = &["B", "KB", "MB", "GB", "TB"];

    if bytes == 0 {
        return "0 B".to_string();
    }

    let bytes = bytes as f64;
    let base = 1024_f64;
    let index = (bytes.ln() / base.ln()).floor() as usize;
    let index = index.min(UNITS.len() - 1);

    let size = bytes / base.powi(index as i32);

    if index == 0 {
        format!("{:.0} {}", size, UNITS[index])
    } else {
        format!("{:.1} {}", size, UNITS[index])
    }
}