# Kanuni CLI (NPM Package)

This is the official npm package for the Kanuni CLI - AI-powered legal intelligence tool.

## Installation

```bash
npm install -g @v-lawyer/kanuni
```

## Usage

After installation, you can use the `kanuni` command:

```bash
kanuni --help
kanuni login
kanuni chat "What are the key points in this contract?"
kanuni analyze document.pdf
```

## What This Package Does

This npm package is a thin wrapper that:
1. Downloads the appropriate Kanuni binary for your platform
2. Installs it in a local `bin` directory
3. Provides a Node.js wrapper to execute the binary

The actual CLI is written in Rust for maximum performance and is compiled to native code for each platform.

## Supported Platforms

- macOS (Intel x64)
- macOS (Apple Silicon ARM64)
- Linux (x64)
- Linux (ARM64)
- Windows (x64)

## Documentation

Full documentation is available at: https://docs.v-lawyer.ai

## Source Code

The source code for the CLI is available at: https://github.com/v-lawyer/kanuni-cli

## License

MIT OR Apache-2.0

## Support

For issues and support, please visit: https://github.com/v-lawyer/kanuni-cli/issues