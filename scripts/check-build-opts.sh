#!/bin/bash
# Script to verify build optimizations are working correctly

set -e

echo "🔍 Checking build optimization configuration..."
echo ""

# Check if .cargo/config.toml exists
if [ -f ".cargo/config.toml" ]; then
    echo "✅ .cargo/config.toml found"
    echo "   - Codegen units: $(grep -m1 'codegen-units' .cargo/config.toml | awk '{print $3}')"
    echo "   - Pipelining: $(grep -m1 'pipelining' .cargo/config.toml | awk '{print $3}')"
else
    echo "❌ .cargo/config.toml not found"
fi

# Check if Cargo.toml has profile optimizations
if grep -q "\[profile.dev\]" src-tauri/Cargo.toml; then
    echo "✅ Profile optimizations found in src-tauri/Cargo.toml"
else
    echo "❌ Profile optimizations missing in src-tauri/Cargo.toml"
fi

# Check for linker based on OS
OS=$(uname -s)
case "$OS" in
    Linux*)
        if grep -q "fuse-ld=lld" .cargo/config.toml 2>/dev/null; then
            echo "✅ LLD linker configured for Linux"
        else
            echo "⚠️  LLD linker not configured (optional)"
        fi
        ;;
    Darwin*)
        if grep -q "ld64.lld" .cargo/config.toml 2>/dev/null; then
            echo "✅ LLD linker configured for macOS"
        else
            echo "⚠️  LLD linker not configured (optional)"
        fi
        ;;
    MINGW*|MSYS*|CYGWIN*)
        if grep -q "SUBSYSTEM:WINDOWS" .cargo/config.toml 2>/dev/null; then
            echo "✅ Windows linker optimizations found"
        else
            echo "⚠️  Windows linker optimizations not found"
        fi
        ;;
esac

echo ""
echo "📊 Current system info:"
echo "   - OS: $OS"
echo "   - CPU cores: $(nproc 2>/dev/null || sysctl -n hw.ncpu 2>/dev/null || echo 'unknown')"
echo "   - Rust version: $(rustc --version 2>/dev/null || echo 'Rust not installed')"
echo "   - Cargo version: $(cargo --version 2>/dev/null || echo 'Cargo not installed')"

echo ""
echo "💡 To test build performance:"
echo "   - Development: time bun tauri:dev"
echo "   - Release: time bun tauri:build"
echo "   - Clean build: time bun cargo:clean && bun tauri:build"

echo ""
echo "📚 See docs/BUILD_OPTIMIZATIONS.md for more details"
