#!/usr/bin/env bash

# Kanuni CLI Installation Script
# This script automatically detects the platform and installs the appropriate binary

set -e

REPO="v-lawyer/kanuni-cli"
INSTALL_DIR="${INSTALL_DIR:-$HOME/.local/bin}"
BINARY_NAME="kanuni"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Helper functions
error() {
    echo -e "${RED}Error: $1${NC}" >&2
    exit 1
}

success() {
    echo -e "${GREEN}✓ $1${NC}"
}

info() {
    echo -e "${BLUE}→ $1${NC}"
}

warning() {
    echo -e "${YELLOW}⚠ $1${NC}"
}

# Detect OS and architecture
detect_platform() {
    local os=""
    local arch=""

    # Detect OS
    case "$(uname -s)" in
        Linux*)     os="linux";;
        Darwin*)    os="darwin";;
        CYGWIN*|MINGW*|MSYS*)    os="windows";;
        *)          error "Unsupported operating system: $(uname -s)";;
    esac

    # Detect architecture
    case "$(uname -m)" in
        x86_64|amd64)   arch="x64";;
        aarch64|arm64)  arch="arm64";;
        *)              error "Unsupported architecture: $(uname -m)";;
    esac

    # Special case for macOS Apple Silicon
    if [[ "$os" == "darwin" && "$arch" == "arm64" ]]; then
        arch="arm64"
    fi

    echo "${os}-${arch}"
}

# Get the latest release version
get_latest_version() {
    local latest_url="https://api.github.com/repos/$REPO/releases/latest"

    if command -v curl > /dev/null; then
        curl -s "$latest_url" | grep '"tag_name":' | sed -E 's/.*"([^"]+)".*/\1/'
    elif command -v wget > /dev/null; then
        wget -qO- "$latest_url" | grep '"tag_name":' | sed -E 's/.*"([^"]+)".*/\1/'
    else
        error "Neither curl nor wget found. Please install one of them."
    fi
}

# Download and extract binary
download_binary() {
    local version="$1"
    local platform="$2"
    local temp_dir="$(mktemp -d)"

    # Determine file extension
    local ext="tar.gz"
    if [[ "$platform" == *"windows"* ]]; then
        ext="zip"
    fi

    local filename="${BINARY_NAME}-${platform}.${ext}"
    local download_url="https://github.com/$REPO/releases/download/${version}/${filename}"

    info "Downloading Kanuni CLI ${version} for ${platform}..."

    # Download the file
    if command -v curl > /dev/null; then
        curl -L -o "$temp_dir/$filename" "$download_url" || error "Failed to download binary"
    elif command -v wget > /dev/null; then
        wget -O "$temp_dir/$filename" "$download_url" || error "Failed to download binary"
    fi

    # Extract the binary
    info "Extracting binary..."
    cd "$temp_dir"

    if [[ "$ext" == "zip" ]]; then
        if command -v unzip > /dev/null; then
            unzip -q "$filename" || error "Failed to extract zip file"
        else
            error "unzip command not found. Please install unzip."
        fi
    else
        tar -xzf "$filename" || error "Failed to extract tar file"
    fi

    # Find the binary (it might be in a subdirectory)
    local binary_path=""
    if [[ -f "$BINARY_NAME" ]]; then
        binary_path="$BINARY_NAME"
    elif [[ -f "$BINARY_NAME.exe" ]]; then
        binary_path="$BINARY_NAME.exe"
    else
        # Search for the binary in subdirectories
        binary_path=$(find . -name "$BINARY_NAME" -o -name "$BINARY_NAME.exe" | head -n 1)
    fi

    if [[ -z "$binary_path" ]]; then
        error "Binary not found in archive"
    fi

    echo "$temp_dir/$binary_path"
}

# Install the binary
install_binary() {
    local binary_path="$1"

    # Create install directory if it doesn't exist
    mkdir -p "$INSTALL_DIR"

    # Copy binary to install directory
    info "Installing to $INSTALL_DIR/$BINARY_NAME..."
    cp "$binary_path" "$INSTALL_DIR/$BINARY_NAME"

    # Make it executable
    chmod +x "$INSTALL_DIR/$BINARY_NAME"

    success "Kanuni CLI installed successfully!"
}

# Check if install directory is in PATH
check_path() {
    if [[ ":$PATH:" != *":$INSTALL_DIR:"* ]]; then
        warning "Installation directory $INSTALL_DIR is not in your PATH"
        echo ""
        echo "Add the following line to your shell configuration file (.bashrc, .zshrc, etc.):"
        echo ""
        echo "  export PATH=\"$INSTALL_DIR:\$PATH\""
        echo ""
        echo "Then reload your shell configuration:"
        echo "  source ~/.bashrc  # or ~/.zshrc"
        echo ""
    fi
}

# Main installation flow
main() {
    echo ""
    echo "╔═══════════════════════════════════════╗"
    echo "║     Kanuni CLI Installation Script     ║"
    echo "╚═══════════════════════════════════════╝"
    echo ""

    # Check for existing installation
    if command -v "$BINARY_NAME" > /dev/null 2>&1; then
        local current_version=$("$BINARY_NAME" --version 2>/dev/null | grep -oE '[0-9]+\.[0-9]+\.[0-9]+' || echo "unknown")
        warning "Kanuni CLI is already installed (version: $current_version)"
        read -p "Do you want to reinstall/update? (y/N): " -n 1 -r
        echo
        if [[ ! $REPLY =~ ^[Yy]$ ]]; then
            exit 0
        fi
    fi

    # Detect platform
    info "Detecting platform..."
    local platform=$(detect_platform)
    success "Detected platform: $platform"

    # Get latest version
    info "Getting latest version..."
    local version=$(get_latest_version)

    if [[ -z "$version" ]]; then
        error "Failed to get latest version"
    fi

    success "Latest version: $version"

    # Download binary
    local binary_path=$(download_binary "$version" "$platform")

    # Install binary
    install_binary "$binary_path"

    # Cleanup
    rm -rf "$(dirname "$binary_path")"

    # Check PATH
    check_path

    # Verify installation
    if "$INSTALL_DIR/$BINARY_NAME" --version > /dev/null 2>&1; then
        echo ""
        success "Installation complete!"
        echo ""
        echo "Run 'kanuni --help' to get started"
        echo ""
    else
        error "Installation verification failed"
    fi
}

# Run the installation
main "$@"