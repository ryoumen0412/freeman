#!/bin/sh
set -e

# Freeman TUI Installer Script
# Usage: curl -fsSL https://raw.githubusercontent.com/ryoumen0412/freeman/main/install.sh | sh

REPO="ryoumen0412/freeman"

echo "🔍 Detecting architecture and operating system..."

OS="$(uname -s)"
ARCH="$(uname -m)"

case "$OS" in
    Linux)
        OS_TYPE="unknown-linux-musl"
        ;;
    Darwin)
        case "$ARCH" in
            x86_64)
                OS_TYPE="apple-darwin"
                ;;
            arm64|aarch64)
                OS_TYPE="apple-darwin"
                ARCH="aarch64"
                ;;
            *)
                echo "❌ Unsupported macOS architecture: $ARCH"
                exit 1
                ;;
        esac
        ;;
    *)
        echo "❌ Unsupported operating system: $OS"
        exit 1
        ;;
esac

TARGET="${ARCH}-${OS_TYPE}"

echo "📦 Target platform: $TARGET"

# Fetch latest release tag
LATEST_TAG=$(curl -sSL "https://api.github.com/repos/$REPO/releases/latest" | grep '"tag_name":' | sed -E 's/.*"([^"]+)".*/\1/')

if [ -z "$LATEST_TAG" ]; then
    echo "❌ Failed to fetch latest release version from GitHub."
    exit 1
fi

echo "🚀 Downloading Freeman $LATEST_TAG for $TARGET..."

ARCHIVE_NAME="freeman-${LATEST_TAG}-${TARGET}.tar.gz"
DOWNLOAD_URL="https://github.com/$REPO/releases/download/$LATEST_TAG/$ARCHIVE_NAME"

TMP_DIR=$(mktemp -d)
trap 'rm -rf "$TMP_DIR"' EXIT

curl -sSL "$DOWNLOAD_URL" -o "$TMP_DIR/$ARCHIVE_NAME"

cd "$TMP_DIR"
tar -xzf "$ARCHIVE_NAME"

# Determine installation directory
if [ -w "/usr/local/bin" ]; then
    INSTALL_DIR="/usr/local/bin"
else
    INSTALL_DIR="$HOME/.local/bin"
    mkdir -p "$INSTALL_DIR"
fi

echo "💾 Installing freeman binary to $INSTALL_DIR..."
mv freeman "$INSTALL_DIR/freeman"
chmod +x "$INSTALL_DIR/freeman"

echo "✅ Freeman $LATEST_TAG successfully installed to $INSTALL_DIR/freeman!"
if ! echo "$PATH" | grep -q "$INSTALL_DIR"; then
    echo "⚠️ Note: $INSTALL_DIR is not currently in your \$PATH."
    echo "   Add it to your shell config (~/.bashrc or ~/.zshrc): export PATH=\"$INSTALL_DIR:\$PATH\""
fi
