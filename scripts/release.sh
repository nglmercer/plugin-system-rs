#!/bin/bash
set -euo pipefail

VERSION=${1:-$(git describe --tags --always --dirty)}
RELEASE_DIR="releases/$VERSION"

echo "=== Building StreamDeck Core $VERSION ==="

mkdir -p "$RELEASE_DIR"

build_platform() {
    local platform=$1
    local target=$2
    local ext=$3
    local plugin_ext=$4
    local archive=$5

    echo "Building $platform..."

    local target_dir="target/$target/release"
    if [ "$target" = "x86_64-unknown-linux-gnu" ] || [ "$target" = "x86_64-pc-windows-gnu" ] || [ "$target" = "x86_64-apple-darwin" ]; then
        target_dir="target/release"
    fi

    local build_dir="$RELEASE_DIR/$platform"
    mkdir -p "$build_dir/plugins"

    # Build web
    echo "  Building web frontend..."
    cd web && npm ci && npm run build && cd ..
    cp -r web/dist "$build_dir/web"

    # Build Rust
    echo "  Building Rust binaries..."
    if [ "$platform" = "linux-arm64" ]; then
        PKG_CONFIG_ALLOW_CROSS=1 \
        PKG_CONFIG_SYSROOT_DIR=/usr/aarch64-linux-gnu \
        PKG_CONFIG_LIBDIR=/usr/aarch64-linux-gnu/usr/lib/pkgconfig:/usr/aarch64-linux-gnu/usr/share/pkgconfig \
        cargo build --release --target "$target"
    else
        cargo build --release --target "$target"
    fi

    # Copy binaries
    cp "$target_dir/sd-core$ext" "$build_dir/"
    cp "$target_dir/libplugin_"*".$plugin_ext" "$build_dir/plugins/" 2>/dev/null || true
    cp "$target_dir/plugin_"*".$plugin_ext" "$build_dir/plugins/" 2>/dev/null || true

    # Create archive
    echo "  Creating archive..."
    cd "$RELEASE_DIR"
    if [ "$ext" = ".exe" ]; then
        zip -r "$archive" "$platform/"
    else
        tar czf "$archive" "$platform/"
    fi
    cd ../..

    echo "  ✓ $platform complete"
}

# Linux x64
build_platform "linux-x64" "x86_64-unknown-linux-gnu" "" "so" "streamdeck-linux-x64.tar.gz"

# Linux ARM64 (requires cross-compilation setup)
if command -v aarch64-linux-gnu-gcc &> /dev/null; then
    build_platform "linux-arm64" "aarch64-unknown-linux-gnu" "" "so" "streamdeck-linux-arm64.tar.gz"
else
    echo "⚠ Skipping linux-arm64 (aarch64-linux-gnu-gcc not found)"
fi

# Windows x64 (requires cross-compilation or Windows host)
if [[ "$OSTYPE" == "msys" ]] || [[ "$OSTYPE" == "win32" ]]; then
    build_platform "windows-x64" "x86_64-pc-windows-gnu" ".exe" "dll" "streamdeck-windows-x64.zip"
else
    echo "⚠ Skipping windows-x64 (use Windows host or cargo-zigbuild)"
fi

# macOS (requires macOS host)
if [[ "$OSTYPE" == "darwin"* ]]; then
    build_platform "macos-x64" "x86_64-apple-darwin" "" "dylib" "streamdeck-macos-x64.tar.gz"
    build_platform "macos-arm64" "aarch64-apple-darwin" "" "dylib" "streamdeck-macos-arm64.tar.gz"
else
    echo "⚠ Skipping macOS targets (use macOS host)"
fi

echo ""
echo "=== Release $VERSION built in $RELEASE_DIR ==="
ls -lh "$RELEASE_DIR"/*.tar.gz "$RELEASE_DIR"/*.zip 2>/dev/null || true
