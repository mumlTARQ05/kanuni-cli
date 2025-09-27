# Changelog

All notable changes to Kanuni CLI will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- Production-grade multi-platform distribution system
- GitHub Actions CI/CD pipeline for automated releases
- NPM package wrapper for Node.js ecosystem
- Docker support with multi-architecture images
- Universal install script for macOS/Linux
- Homebrew tap support
- Automated changelog generation
- Release optimization with LTO and symbol stripping

### Changed
- Replaced keyring-based authentication with OAuth Device Flow and API Keys
- Improved security with file-based token storage
- Enhanced binary size optimization

### Security
- Removed dependency on system keyring
- Implemented secure token storage with proper file permissions

## [0.1.0] - 2024-01-XX

### Added
- Initial release of Kanuni CLI
- Document analysis capabilities
- AI chat assistant
- Case law search functionality
- OAuth Device Flow authentication
- API Key authentication
- Beautiful terminal UI with colors and progress indicators
- Multi-platform support (macOS, Linux, Windows)

[Unreleased]: https://github.com/v-lawyer/kanuni-cli/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/v-lawyer/kanuni-cli/releases/tag/v0.1.0