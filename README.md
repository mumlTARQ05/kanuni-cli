# Kanuni - AI-Powered Legal Intelligence CLI

<div align="center">
  <img src="assets/logo.png" alt="Kanuni Logo" width="200" />
  <h3>The Ottoman Edition</h3>
  <p>Named after Suleiman the Lawgiver (Kanuni Sultan Süleyman)</p>

  [![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
  [![License: Apache 2.0](https://img.shields.io/badge/License-Apache%202.0-blue.svg)](https://opensource.org/licenses/Apache-2.0)
  [![Rust](https://img.shields.io/badge/rust-%23000000.svg?style=flat&logo=rust&logoColor=white)](https://www.rust-lang.org/)
  [![Crates.io](https://img.shields.io/crates/v/kanuni.svg)](https://crates.io/crates/kanuni)
</div>

## 🚀 Features

- **📄 Document Analysis** - Extract key information, dates, parties, and risks from legal documents
- **💬 AI Chat Assistant** - Interactive legal guidance powered by advanced language models
- **🔍 Case Law Search** - Search through legal precedents and case databases
- **📅 Deadline Extraction** - Automatically extract and track important dates
- **🎨 Beautiful CLI** - Intuitive interface with colors and progress indicators
- **🔐 Secure** - API key authentication with secure storage
- **⚡ Fast** - Built with Rust for maximum performance

## 📦 Installation

### Via Cargo (Recommended)

```bash
cargo install kanuni
```

### Via Homebrew (macOS/Linux)

```bash
brew tap v-lawyer/tap
brew install kanuni
```

### From Source

```bash
git clone https://github.com/v-lawyer/kanuni-cli.git
cd kanuni-cli
cargo build --release
sudo mv target/release/kanuni /usr/local/bin/
```

## 🔧 Configuration

First, authenticate with your V-Lawyer API key:

```bash
kanuni auth login
```

Don't have an API key? Sign up at [v-lawyer.ai](https://v-lawyer.ai)

## 📖 Usage

### Document Analysis

Analyze legal documents to extract key information:

```bash
# Basic analysis
kanuni analyze contract.pdf

# Extract specific information
kanuni analyze contract.pdf -e dates -e parties -e obligations

# Output as JSON
kanuni analyze contract.pdf --format json
```

### AI Chat Assistant

Get instant legal guidance:

```bash
# Start interactive chat
kanuni chat

# Ask a specific question
kanuni chat "What are the key elements of a valid contract?"

# Chat with document context
kanuni chat -d contract.pdf "What are the risks in this agreement?"

# Continue previous session
kanuni chat --session abc123
```

### Case Law Search

Search through legal precedents:

```bash
# Basic search
kanuni search "negligence duty of care"

# Filter by jurisdiction
kanuni search "contract breach" -j "California"

# Limit results and date range
kanuni search "intellectual property" -n 20 -d "2020-2024"
```

### Date & Deadline Extraction

Extract important dates from documents:

```bash
# Extract from single document
kanuni extract contract.pdf

# Extract from directory
kanuni extract ./legal-docs/

# Export as calendar file
kanuni extract contract.pdf --format ical

# Add reminders
kanuni extract contract.pdf --reminder 7
```

## 🛠️ Advanced Usage

### Shell Completions

Generate completions for your shell:

```bash
# Bash
kanuni completions bash > /usr/local/share/bash-completion/completions/kanuni

# Zsh
kanuni completions zsh > /usr/local/share/zsh/site-functions/_kanuni

# Fish
kanuni completions fish > ~/.config/fish/completions/kanuni.fish
```

### Configuration Management

```bash
# Show current config
kanuni config show

# Set custom API endpoint
kanuni config set api_endpoint https://custom.api.endpoint

# Reset to defaults
kanuni config reset
```

## 🧪 Development

### Prerequisites

- Rust 1.70+
- Cargo

### Building

```bash
# Clone the repository
git clone https://github.com/v-lawyer/kanuni-cli.git
cd kanuni-cli

# Build debug version
cargo build

# Run tests
cargo test

# Build optimized release
cargo build --release
```

### Project Structure

```
kanuni-cli/
├── src/
│   ├── main.rs           # Entry point
│   ├── cli.rs           # CLI argument parsing
│   ├── commands/        # Command implementations
│   ├── api.rs           # API client
│   ├── config.rs        # Configuration management
│   └── utils/           # Utilities and helpers
├── Cargo.toml           # Dependencies
└── README.md           # This file
```

## 🤝 Contributing

We welcome contributions! Please see our [Contributing Guide](CONTRIBUTING.md) for details.

### How to Contribute

1. Fork the repository
2. Create a feature branch (`git checkout -b feature/amazing-feature`)
3. Commit your changes (`git commit -m 'Add amazing feature'`)
4. Push to the branch (`git push origin feature/amazing-feature`)
5. Open a Pull Request

## 📝 License

This project is dual-licensed under either:

- MIT License ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)
- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)

at your option.

## 🙏 Acknowledgments

- Named after [Suleiman the Magnificent](https://en.wikipedia.org/wiki/Suleiman_the_Magnificent), known as "Kanuni" (The Lawgiver)
- Built with [Rust](https://www.rust-lang.org/) for performance and safety
- Powered by [V-Lawyer](https://v-lawyer.ai) API

## 📞 Support

- **Documentation**: [docs.v-lawyer.ai](https://docs.v-lawyer.ai)
- **Issues**: [GitHub Issues](https://github.com/v-lawyer/kanuni-cli/issues)
- **Email**: support@v-lawyer.ai
- **Twitter**: [@vlawyer](https://twitter.com/vlawyer)

## 🚦 Status

[![CI](https://github.com/v-lawyer/kanuni-cli/workflows/CI/badge.svg)](https://github.com/v-lawyer/kanuni-cli/actions)
[![Coverage](https://codecov.io/gh/v-lawyer/kanuni-cli/branch/main/graph/badge.svg)](https://codecov.io/gh/v-lawyer/kanuni-cli)

---

<div align="center">
  Made with ❤️ by the V-Lawyer Team
  <br>
  <sub>Building the future of legal technology</sub>
</div>