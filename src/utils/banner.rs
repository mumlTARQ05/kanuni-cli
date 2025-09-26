use colored::*;

pub fn display() {
    let banner = r#"
╔══════════════════════════════════════════════════════════════════════╗
║                                                                      ║
║  ██╗  ██╗ █████╗ ███╗   ██╗██╗   ██╗███╗   ██╗██╗                  ║
║  ██║ ██╔╝██╔══██╗████╗  ██║██║   ██║████╗  ██║██║                  ║
║  █████╔╝ ███████║██╔██╗ ██║██║   ██║██╔██╗ ██║██║                  ║
║  ██╔═██╗ ██╔══██║██║╚██╗██║██║   ██║██║╚██╗██║██║                  ║
║  ██║  ██╗██║  ██║██║ ╚████║╚██████╔╝██║ ╚████║██║                  ║
║  ╚═╝  ╚═╝╚═╝  ╚═╝╚═╝  ╚═══╝ ╚═════╝ ╚═╝  ╚═══╝╚═╝                  ║
║                                                                      ║
║            The Legal Intelligence CLI • Ottoman Edition              ║
║                    Named after Suleiman the Lawgiver                ║
║                                                                      ║
╚══════════════════════════════════════════════════════════════════════╝
"#;

    println!("{}", banner.cyan().bold());

    // Version and build info
    let version = env!("CARGO_PKG_VERSION");
    println!("  {} v{}", "⚖️  Kanuni".yellow().bold(), version);
    println!("  {} {}", "📜 Type".white(), "kanuni --help".green().bold(), );
    println!("  {} {}\n", "🏛️  Docs".white(), "https://github.com/v-lawyer/kanuni-cli".blue());
}