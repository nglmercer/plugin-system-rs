#!/bin/bash
set -euo pipefail

VERSION=${1:-$(git describe --tags --always --dirty)}
RELEASE_DIR="releases/$VERSION"

echo "=== Building StreamDeck Core $VERSION ==="

# Build the CLI tool first
echo "Building sd-plugins CLI..."
cargo build --release -p sd-plugins-cli 2>/dev/null || cargo build -p sd-plugins-cli

CLI="target/release/sd-plugins"
if [ ! -f "$CLI" ]; then
    CLI="target/debug/sd-plugins"
fi

# Build all plugins for host platform
echo ""
echo "Building plugins for host platform..."
$CLI build --release --with-web --with-core

# Package for release
echo ""
echo "Packaging release..."
$CLI package --version "$VERSION" --output releases

echo ""
echo "=== Release $VERSION complete ==="
ls -lh "$RELEASE_DIR"/*.tar.gz "$RELEASE_DIR"/*.zip 2>/dev/null || true
