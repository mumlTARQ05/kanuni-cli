use anyhow::Result;
use clap::{Parser, Subcommand};

use crate::commands;

#[derive(Parser)]
#[command(
    name = "kanuni",
    author = "V-Lawyer Team",
    version,
    about = "AI-powered legal intelligence CLI - The Ottoman Edition",
    long_about = "Kanuni brings the wisdom of Suleiman the Lawgiver to your terminal.\nAnalyze documents, search case law, and get AI-powered legal assistance."
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,

    /// Enable verbose output
    #[arg(short, long, global = true)]
    pub verbose: bool,

    /// Configuration file path
    #[arg(short, long, global = true)]
    pub config: Option<String>,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Analyze legal documents for key information
    Analyze {
        /// Path to the document to analyze (or use --document-id for existing documents)
        #[arg(value_name = "FILE", conflicts_with = "document_id")]
        file: Option<String>,

        /// Document ID to analyze (for already uploaded documents)
        #[arg(long, conflicts_with = "file")]
        document_id: Option<String>,

        /// Output format (json, text, markdown)
        #[arg(short, long, default_value = "text")]
        format: String,

        /// Extract specific information (dates, parties, obligations, risks)
        #[arg(short = 'e', long)]
        extract: Vec<String>,
    },

    /// Interactive chat with legal AI assistant
    Chat {
        /// Initial message or question
        #[arg(value_name = "MESSAGE")]
        message: Option<String>,

        /// Reference a document for context
        #[arg(short = 'd', long)]
        document: Option<String>,

        /// Continue previous chat session
        #[arg(short = 's', long)]
        session: Option<String>,
    },

    /// Search case law and legal precedents
    Search {
        /// Search query
        #[arg(value_name = "QUERY")]
        query: String,

        /// Jurisdiction filter
        #[arg(short = 'j', long)]
        jurisdiction: Option<String>,

        /// Date range (e.g., "2020-2024")
        #[arg(short = 'd', long)]
        date_range: Option<String>,

        /// Maximum number of results
        #[arg(short = 'n', long, default_value = "10")]
        limit: usize,
    },

    /// Extract important dates and deadlines
    Extract {
        /// Path to document or directory
        #[arg(value_name = "PATH")]
        path: String,

        /// Output format (ical, json, csv)
        #[arg(short, long, default_value = "json")]
        format: String,

        /// Include reminders (days before deadline)
        #[arg(short = 'r', long)]
        reminder: Option<u32>,
    },

    /// Configure Kanuni settings
    Config {
        #[command(subcommand)]
        action: ConfigAction,
    },

    /// Authenticate with V-Lawyer API
    Auth {
        #[command(subcommand)]
        action: AuthAction,
    },

    /// Generate shell completions
    Completions {
        /// Shell to generate completions for
        #[arg(value_enum)]
        shell: clap_complete::Shell,
    },

    /// Manage documents
    Document {
        #[command(subcommand)]
        action: DocumentAction,
    },
}

#[derive(Subcommand)]
pub enum ConfigAction {
    /// Show current configuration
    Show,
    /// Set a configuration value
    Set {
        /// Configuration key
        key: String,
        /// Configuration value
        value: String,
    },
    /// Reset configuration to defaults
    Reset,
}

#[derive(Subcommand)]
pub enum AuthAction {
    /// Login to V-Lawyer
    Login {
        /// Use API key for authentication (will use device flow if not provided)
        #[arg(long = "api-key")]
        api_key: Option<String>,
    },
    /// Logout from V-Lawyer
    Logout,
    /// Check authentication status
    Status,
    /// Create a new API key
    #[command(name = "create-key")]
    CreateKey,
    /// List all API keys
    #[command(name = "list-keys")]
    ListKeys,
}

#[derive(Subcommand)]
pub enum DocumentAction {
    /// Upload a document without analysis
    Upload {
        /// Path to the document to upload
        file: String,
        /// Document category (legal, contract, financial, medical, personal, other)
        #[arg(long)] // Removed short flag to avoid conflict with global -c config
        category: Option<String>,
        /// Document description
        #[arg(short, long)]
        description: Option<String>,
    },
    /// List all documents
    List {
        /// Maximum number of documents to show
        #[arg(short, long)]
        limit: Option<i32>,
        /// Number of documents to skip
        #[arg(short, long)]
        offset: Option<i32>,
    },
    /// Show document details
    Info {
        /// Document ID (full UUID or first 8 characters)
        id: String,
    },
    /// Delete a document
    Delete {
        /// Document ID (full UUID or first 8 characters)
        id: String,
    },
    /// Download a document
    Download {
        /// Document ID (full UUID or first 8 characters)
        id: String,
        /// Output file path (defaults to original filename)
        #[arg(short, long)]
        output: Option<String>,
    },
}

impl Cli {
    pub async fn execute(&self) -> Result<()> {
        match &self.command {
            Commands::Analyze {
                file,
                document_id,
                format,
                extract,
            } => {
                commands::analyze::execute(file.as_deref(), document_id.as_deref(), format, extract)
                    .await
            }
            Commands::Chat {
                message,
                document,
                session,
            } => {
                commands::chat::execute(message.as_deref(), document.as_deref(), session.as_deref())
                    .await
            }
            Commands::Search {
                query,
                jurisdiction,
                date_range,
                limit,
            } => {
                commands::search::execute(
                    query,
                    jurisdiction.as_deref(),
                    date_range.as_deref(),
                    *limit,
                )
                .await
            }
            Commands::Extract {
                path,
                format,
                reminder,
            } => commands::extract::execute(path, format, *reminder).await,
            Commands::Config { action } => commands::config::execute(action).await,
            Commands::Auth { action } => commands::auth::execute(action).await,
            Commands::Completions { shell } => commands::completions::execute(*shell),
            Commands::Document { action } => commands::document::execute(action).await,
        }
    }
}
