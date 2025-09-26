use anyhow::{Result, Context, bail};
use colored::*;
use std::path::Path;
use uuid::Uuid;
use crate::config::Config;
use crate::api::{ApiClient, AnalysisType, DocumentCategory};

pub async fn execute(file: Option<&str>, document_id: Option<&str>, _format: &str, extract: &[String]) -> Result<()> {
    // Load config and create API client
    let config = Config::load()?;
    let api_client = ApiClient::new(config)?;

    // Determine analysis type based on extract options
    let analysis_type = determine_analysis_type(extract);

    let result = if let Some(doc_id) = document_id {
        // Analyze existing document
        println!("{}  {}", "📄".cyan(), format!("Analyzing document ID: {}", doc_id).bold());

        let uuid = Uuid::parse_str(doc_id)
            .context("Invalid document ID format")?;

        api_client
            .analyze_existing_document(uuid, analysis_type)
            .await
            .context("Failed to analyze document")?
    } else if let Some(file_path) = file {
        // Upload and analyze new document
        println!("{}  {}", "📄".cyan(), format!("Analyzing document: {}", file_path).bold());

        // Determine document category from file extension or name
        let category = determine_category(file_path);

        // Upload and analyze document
        let path = Path::new(file_path);
        if !path.exists() {
            bail!("File not found: {}", file_path);
        }

        api_client
            .upload_and_analyze(path, analysis_type, category)
            .await
            .context("Failed to analyze document")?
    } else {
        bail!("Please provide either a file path or --document-id");
    };

    // Display results
    display_results(&result)?;

    Ok(())
}

fn determine_analysis_type(extract: &[String]) -> AnalysisType {
    // Determine analysis type based on what user wants to extract
    if extract.iter().any(|e| e.contains("legal") || e.contains("risk")) {
        AnalysisType::Legal
    } else if extract.iter().any(|e| e.contains("financial") || e.contains("money")) {
        AnalysisType::Financial
    } else if extract.iter().any(|e| e.contains("medical") || e.contains("health")) {
        AnalysisType::Medical
    } else if extract.is_empty() {
        AnalysisType::Quick
    } else {
        AnalysisType::Detailed
    }
}

fn determine_category(file_path: &str) -> Option<DocumentCategory> {
    let lower_path = file_path.to_lowercase();

    if lower_path.contains("contract") || lower_path.contains("agreement") {
        Some(DocumentCategory::Contract)
    } else if lower_path.contains("legal") || lower_path.contains("lawsuit") {
        Some(DocumentCategory::Legal)
    } else if lower_path.contains("financial") || lower_path.contains("invoice") {
        Some(DocumentCategory::Financial)
    } else if lower_path.contains("medical") || lower_path.contains("health") {
        Some(DocumentCategory::Medical)
    } else {
        None
    }
}

fn display_results(result: &crate::api::AnalysisResultResponse) -> Result<()> {
    println!("\n{}", "📊 Analysis Results:".green().bold());
    println!("  Analysis ID: {}", result.id.to_string().yellow());
    println!("  Type: {}", format!("{:?}", result.analysis_type).yellow());

    if let Some(processing_time) = result.processing_time_ms {
        let seconds = processing_time as f64 / 1000.0;
        println!("  Processing time: {:.2}s", seconds);
    }

    if let Some(summary) = &result.summary {
        println!("\n{}", "📝 Summary:".green().bold());
        for line in summary.lines() {
            println!("  {}", line);
        }
    }

    if let Some(findings) = &result.key_findings {
        if !findings.is_empty() {
            println!("\n{}", "🔍 Key Findings:".green().bold());
            for finding in findings {
                println!("  • {}", finding);
            }
        }
    }

    if let Some(risk) = &result.risk_assessment {
        println!("\n{}", "⚠️ Risk Assessment:".yellow().bold());
        let level_color = match risk.level.to_lowercase().as_str() {
            "high" => risk.level.red().bold(),
            "medium" => risk.level.yellow().bold(),
            "low" => risk.level.green().bold(),
            _ => risk.level.white().bold(),
        };
        println!("  Risk Level: {}", level_color);

        if !risk.factors.is_empty() {
            println!("\n  Risk Factors:");
            for factor in &risk.factors {
                println!("    • {}", factor);
            }
        }

        if !risk.recommendations.is_empty() {
            println!("\n  Recommendations:");
            for rec in &risk.recommendations {
                println!("    ✓ {}", rec.green());
            }
        }
    }

    if let Some(entities) = &result.entities {
        if !entities.is_empty() {
            println!("\n{}", "👥 Extracted Entities:".blue().bold());
            for entity in entities {
                println!("  • {}: {} (confidence: {:.0}%)",
                    entity.entity_type.cyan(),
                    entity.value.yellow(),
                    entity.confidence * 100.0
                );
            }
        }
    }

    if let Some(dates) = &result.dates {
        if !dates.is_empty() {
            println!("\n{}", "📅 Important Dates:".blue().bold());
            for date in dates {
                println!("  • {} - {} ({})",
                    date.date.yellow(),
                    date.context,
                    date.date_type.cyan()
                );
            }
        }
    }

    Ok(())
}