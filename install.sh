#!/usr/bin/env bash

set -e

# Kanuni CLI Installation Script
# This script installs the Kanuni CLI on macOS, Linux, and WSL

REPO="v-lawyer/kanuni-cli"
INSTALL_DIR="${KANUNI_INSTALL_DIR:-/usr/local/bin}"
BINARY_NAME="kanuni"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Logging functions
info() {
    echo -e "${BLUE}ℹ${NC} $1"
}

success() {
    echo -e "${GREEN}✓${NC} $1"
}

error() {
    echo -e "${RED}✗${NC} $1" >&2
}

warning() {
    echo -e "${YELLOW}⚠${NC} $1"
}

# Detect OS and architecture
detect_platform() {
    local os
    local arch

    # Detect OS
    case "$(uname -s)" in
        Darwin*)    os="darwin" ;;
        Linux*)     os="linux" ;;
        MINGW*|CYGWIN*|MSYS*)
            error "Windows is not directly supported. Please use WSL or download from GitHub releases."
            exit 1
            ;;
        *)
            error "Unsupported operating system: $(uname -s)"
            exit 1
            ;;
    esac

    # Detect architecture
    case "$(uname -m)" in
        x86_64|amd64)   arch="x64" ;;
        arm64|aarch64)  arch="arm64" ;;
        *)
            error "Unsupported architecture: $(uname -m)"
            exit 1
            ;;
    esac

    echo "${os}-${arch}"
}

# Get the latest release version from GitHub
get_latest_version() {
    local version
    version=$(curl -s "https://api.github.com/repos/${REPO}/releases/latest" | grep '"tag_name":' | sed -E 's/.*"([^"]+)".*/\1/')

    if [ -z "$version" ]; then
        error "Failed to fetch the latest version"
        exit 1
    fi

    echo "$version"
}

# Download and verify the binary
download_binary() {
    local version=$1
    local platform=$2
    local binary_name="kanuni-${platform}"
    local download_url="https://github.com/${REPO}/releases/download/${version}/${binary_name}.tar.gz"
    local checksum_url="${download_url}.sha256"
    local temp_dir

    temp_dir=$(mktemp -d)
    cd "$temp_dir"

    info "Downloading Kanuni CLI ${version} for ${platform}..."

    # Download the binary archive
    if ! curl -L --progress-bar -o "${binary_name}.tar.gz" "$download_url"; then
        error "Failed to download binary"
        exit 1
    fi

    # Download and verify checksum
    info "Verifying checksum..."
    curl -sL -o "${binary_name}.tar.gz.sha256" "$checksum_url"

    # Verify checksum
    if command -v sha256sum > /dev/null 2>&1; then
        if ! sha256sum -c "${binary_name}.tar.gz.sha256" > /dev/null 2>&1; then
            error "Checksum verification failed"
            exit 1
        fi
    elif command -v shasum > /dev/null 2>&1; then
        expected=$(cat "${binary_name}.tar.gz.sha256" | awk '{print $1}')
        actual=$(shasum -a 256 "${binary_name}.tar.gz" | awk '{print $1}')
        if [ "$expected" != "$actual" ]; then
            error "Checksum verification failed"
            exit 1
        fi
    else
        warning "Cannot verify checksum (no sha256sum or shasum available)"
    fi

    success "Checksum verified"

    # Extract the binary
    info "Extracting binary..."
    tar -xzf "${binary_name}.tar.gz"

    # Install the binary
    info "Installing to ${INSTALL_DIR}..."

    # Check if we need sudo
    if [ -w "$INSTALL_DIR" ]; then
        mv "$binary_name" "${INSTALL_DIR}/${BINARY_NAME}"
    else
        warning "Sudo required to install to ${INSTALL_DIR}"
        sudo mv "$binary_name" "${INSTALL_DIR}/${BINARY_NAME}"
    fi

    # Make executable
    if [ -w "${INSTALL_DIR}/${BINARY_NAME}" ]; then
        chmod +x "${INSTALL_DIR}/${BINARY_NAME}"
    else
        sudo chmod +x "${INSTALL_DIR}/${BINARY_NAME}"
    fi

    # Clean up
    cd - > /dev/null
    rm -rf "$temp_dir"
}

# Setup shell completions
setup_completions() {
    info "Setting up shell completions..."

    local shell_name

    # Detect shell
    if [ -n "$BASH_VERSION" ]; then
        shell_name="bash"
    elif [ -n "$ZSH_VERSION" ]; then
        shell_name="zsh"
    elif [ -n "$FISH_VERSION" ]; then
        shell_name="fish"
    else
        warning "Unknown shell. Skipping completion setup."
        return
    fi

    # Generate completions
    if command -v kanuni > /dev/null 2>&1; then
        case "$shell_name" in
            bash)
                if [ -d "/etc/bash_completion.d" ]; then
                    kanuni completions bash | sudo tee /etc/bash_completion.d/kanuni > /dev/null
                    success "Bash completions installed"
                fi
                ;;
            zsh)
                if [ -d "/usr/local/share/zsh/site-functions" ]; then
                    kanuni completions zsh | sudo tee /usr/local/share/zsh/site-functions/_kanuni > /dev/null
                    success "Zsh completions installed"
                fi
                ;;
            fish)
                if [ -d "$HOME/.config/fish/completions" ]; then
                    kanuni completions fish > "$HOME/.config/fish/completions/kanuni.fish"
                    success "Fish completions installed"
                fi
                ;;
        esac
    fi
}

# Main installation flow
main() {
    echo
    echo "╔══════════════════════════════════════════╗"
    echo "║     Kanuni CLI Installation Script       ║"
    echo "║         AI-Powered Legal Intelligence    ║"
    echo "╚══════════════════════════════════════════╝"
    echo

    # Check for required tools
    for tool in curl tar; do
        if ! command -v $tool > /dev/null 2>&1; then
            error "$tool is required but not installed"
            exit 1
        fi
    done

    # Detect platform
    info "Detecting platform..."
    PLATFORM=$(detect_platform)
    success "Platform: $PLATFORM"

    # Get latest version
    info "Fetching latest version..."
    VERSION=$(get_latest_version)
    success "Latest version: $VERSION"

    # Check if already installed
    if command -v kanuni > /dev/null 2>&1; then
        CURRENT_VERSION=$(kanuni --version 2>/dev/null | cut -d' ' -f2 || echo "unknown")
        warning "Kanuni is already installed (version: $CURRENT_VERSION)"
        read -p "Do you want to continue with the installation? (y/N) " -n 1 -r
        echo
        if [[ ! $REPLY =~ ^[Yy]$ ]]; then
            info "Installation cancelled"
            exit 0
        fi
    fi

    # Download and install
    download_binary "$VERSION" "$PLATFORM"

    # Verify installation
    if command -v kanuni > /dev/null 2>&1; then
        success "Kanuni CLI installed successfully!"

        # Setup completions
        setup_completions

        # Show version
        echo
        kanuni --version
        echo

        # Instructions
        echo "🎉 Installation complete!"
        echo
        echo "To get started, run:"
        echo "  kanuni --help"
        echo
        echo "To authenticate, run:"
        echo "  kanuni login"
        echo
        echo "Documentation: https://docs.v-lawyer.ai"
        echo
    else
        error "Installation failed. Kanuni command not found."
        error "Please ensure ${INSTALL_DIR} is in your PATH"
        exit 1
    fi
}

# Run the installation
main "$@"