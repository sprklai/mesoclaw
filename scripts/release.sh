#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"

DRY_RUN=false
VERSION=""

usage() {
    echo "Usage: $0 [--dry-run] <version>"
    echo ""
    echo "Sync version across all project files and create a git tag."
    echo ""
    echo "Arguments:"
    echo "  version      Semver version (e.g., 1.2.0)"
    echo ""
    echo "Options:"
    echo "  --dry-run    Show changes without applying them"
    echo "  -h, --help   Show this help message"
    echo ""
    echo "Examples:"
    echo "  $0 1.2.0"
    echo "  $0 --dry-run 1.2.0"
}

# Parse arguments
while [[ $# -gt 0 ]]; do
    case $1 in
        --dry-run)
            DRY_RUN=true
            shift
            ;;
        -h|--help)
            usage
            exit 0
            ;;
        -*)
            echo "Error: Unknown option $1"
            usage
            exit 1
            ;;
        *)
            VERSION="$1"
            shift
            ;;
    esac
done

if [[ -z "$VERSION" ]]; then
    echo "Error: Version argument required"
    usage
    exit 1
fi

# Validate semver format
if ! [[ "$VERSION" =~ ^[0-9]+\.[0-9]+\.[0-9]+(-[a-zA-Z0-9.]+)?$ ]]; then
    echo "Error: Invalid semver format: $VERSION"
    echo "Expected format: MAJOR.MINOR.PATCH (e.g., 1.2.0)"
    exit 1
fi

CARGO_TOML="$ROOT_DIR/Cargo.toml"
PACKAGE_JSON="$ROOT_DIR/web/package.json"
TAURI_CONF="$ROOT_DIR/crates/mesoclaw-desktop/tauri.conf.json"

echo "=== MesoClaw Release: v$VERSION ==="
echo ""

# --- Cargo.toml (workspace.package.version) ---
CARGO_OLD=$(grep -oP '(?<=^version = ")[^"]+' "$CARGO_TOML" | head -1)
echo "[1/3] Cargo.toml: $CARGO_OLD -> $VERSION"

if [[ "$DRY_RUN" == false ]]; then
    sed -i "0,/^version = \"$CARGO_OLD\"/s//version = \"$VERSION\"/" "$CARGO_TOML"
fi

# --- web/package.json ---
PKG_OLD=$(grep -oP '(?<="version": ")[^"]+' "$PACKAGE_JSON")
echo "[2/3] web/package.json: $PKG_OLD -> $VERSION"

if [[ "$DRY_RUN" == false ]]; then
    sed -i "s/\"version\": \"$PKG_OLD\"/\"version\": \"$VERSION\"/" "$PACKAGE_JSON"
fi

# --- tauri.conf.json ---
TAURI_OLD=$(grep -oP '(?<="version": ")[^"]+' "$TAURI_CONF" | head -1)
echo "[3/3] tauri.conf.json: $TAURI_OLD -> $VERSION"

if [[ "$DRY_RUN" == false ]]; then
    sed -i "s/\"version\": \"$TAURI_OLD\"/\"version\": \"$VERSION\"/" "$TAURI_CONF"
fi

echo ""

if [[ "$DRY_RUN" == true ]]; then
    echo "[dry-run] No files modified. No tag created."
    echo ""
fi

# --- Git tag ---
TAG="app-v$VERSION"
if [[ "$DRY_RUN" == false ]]; then
    echo "Creating git tag: $TAG"
    git -C "$ROOT_DIR" add "$CARGO_TOML" "$PACKAGE_JSON" "$TAURI_CONF"
    git -C "$ROOT_DIR" commit -m "release: v$VERSION"
    git -C "$ROOT_DIR" tag "$TAG"
    echo "Tag $TAG created."
else
    echo "[dry-run] Would create git tag: $TAG"
fi

echo ""

# --- Changelog (commits since last tag) ---
LAST_TAG=$(git -C "$ROOT_DIR" describe --tags --abbrev=0 2>/dev/null || echo "")
echo "=== Changelog ==="
if [[ -n "$LAST_TAG" ]]; then
    echo "Changes since $LAST_TAG:"
    echo ""
    git -C "$ROOT_DIR" log --oneline "$LAST_TAG"..HEAD
else
    echo "No previous tags found. Showing recent commits:"
    echo ""
    git -C "$ROOT_DIR" log --oneline -20
fi

echo ""
echo "Done."
