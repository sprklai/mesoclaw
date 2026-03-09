#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"

# Files to sync
CARGO_TOML="$ROOT_DIR/Cargo.toml"
TAURI_CONF="$ROOT_DIR/crates/mesoclaw-desktop/tauri.conf.json"
PACKAGE_JSON="$ROOT_DIR/web/package.json"
ENV_TS="$ROOT_DIR/web/src/test-mocks/environment.ts"
IDENTITY_TYPES="$ROOT_DIR/crates/mesoclaw-core/src/identity/types.rs"
IDENTITY_MD="$ROOT_DIR/crates/mesoclaw-core/src/identity/defaults/IDENTITY.md"
CHANGELOG="$ROOT_DIR/CHANGELOG.md"

usage() {
    echo "Usage: $0 <patch|minor|major|set VERSION>"
    echo ""
    echo "Examples:"
    echo "  $0 patch          # 0.0.1 → 0.0.2"
    echo "  $0 minor          # 0.0.1 → 0.1.0"
    echo "  $0 major          # 0.0.1 → 1.0.0"
    echo "  $0 set 0.0.1      # Rebaseline to specific version"
    exit 1
}

get_current_version() {
    grep -m1 '^version = "' "$CARGO_TOML" | sed 's/version = "\(.*\)"/\1/'
}

compute_new_version() {
    local current="$1"
    local bump_type="$2"

    local major minor patch
    IFS='.' read -r major minor patch <<< "$current"

    case "$bump_type" in
        patch) patch=$((patch + 1)) ;;
        minor) minor=$((minor + 1)); patch=0 ;;
        major) major=$((major + 1)); minor=0; patch=0 ;;
        *) echo "Error: unknown bump type '$bump_type'"; exit 1 ;;
    esac

    echo "${major}.${minor}.${patch}"
}

update_files() {
    local old="$1"
    local new="$2"

    # 1. Cargo.toml (workspace version)
    sed -i "s/^version = \"${old}\"/version = \"${new}\"/" "$CARGO_TOML"

    # 2. tauri.conf.json
    sed -i "s/\"version\": \"${old}\"/\"version\": \"${new}\"/" "$TAURI_CONF"

    # 3. web/package.json
    sed -i "s/\"version\": \"${old}\"/\"version\": \"${new}\"/" "$PACKAGE_JSON"

    # 4. environment.ts
    sed -i "s/export const version = \"${old}\"/export const version = \"${new}\"/" "$ENV_TS"

    # 5. identity/types.rs (default impl + test assertion)
    sed -i "s/version: \"${old}\".into()/version: \"${new}\".into()/" "$IDENTITY_TYPES"
    sed -i "s/assert_eq!(meta.version, \"${old}\")/assert_eq!(meta.version, \"${new}\")/" "$IDENTITY_TYPES"

    # 6. IDENTITY.md
    sed -i "s/^version: \"${old}\"/version: \"${new}\"/" "$IDENTITY_MD"
}

update_changelog_bump() {
    local new="$1"
    local today
    today=$(date +%Y-%m-%d)

    # Insert new version section after [Unreleased]
    sed -i "/^## \[Unreleased\]/a\\\\n## [${new}] - ${today}" "$CHANGELOG"
}

update_changelog_set() {
    local old="$1"
    local new="$2"

    # Replace existing version in-place
    sed -i "s/^## \[${old}\]/## [${new}]/" "$CHANGELOG"
}

verify() {
    local version="$1"
    local errors=0

    echo ""
    echo "Verifying version $version across all files..."

    check_file() {
        local file="$1"
        local pattern="$2"
        local label="$3"
        if grep -q "$pattern" "$file"; then
            echo "  OK  $label"
        else
            echo "  FAIL  $label"
            errors=$((errors + 1))
        fi
    }

    check_file "$CARGO_TOML"       "^version = \"${version}\"" "Cargo.toml"
    check_file "$TAURI_CONF"       "\"version\": \"${version}\"" "tauri.conf.json"
    check_file "$PACKAGE_JSON"     "\"version\": \"${version}\"" "package.json"
    check_file "$ENV_TS"           "version = \"${version}\"" "environment.ts"
    check_file "$IDENTITY_TYPES"   "version: \"${version}\".into()" "identity/types.rs"
    check_file "$IDENTITY_MD"      "^version: \"${version}\"" "IDENTITY.md"
    check_file "$CHANGELOG"        "## \[${version}\]" "CHANGELOG.md"

    if [ "$errors" -gt 0 ]; then
        echo ""
        echo "WARNING: $errors file(s) failed verification"
        return 1
    fi

    echo ""
    echo "All 7 files verified."
}

# --- Main ---

if [ $# -lt 1 ]; then
    usage
fi

COMMAND="$1"
CURRENT=$(get_current_version)

case "$COMMAND" in
    set)
        if [ $# -lt 2 ]; then
            echo "Error: 'set' requires a version argument"
            usage
        fi
        NEW="$2"
        if ! [[ "$NEW" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]]; then
            echo "Error: invalid version format '$NEW' (expected X.Y.Z)"
            exit 1
        fi
        ;;
    patch|minor|major)
        NEW=$(compute_new_version "$CURRENT" "$COMMAND")
        ;;
    *)
        usage
        ;;
esac

echo "Version bump: $CURRENT → $NEW"
echo ""

update_files "$CURRENT" "$NEW"

if [ "$COMMAND" = "set" ]; then
    update_changelog_set "$CURRENT" "$NEW"
else
    update_changelog_bump "$NEW"
fi

verify "$NEW"

echo ""
echo "Done. Run 'cargo check --workspace' to validate."
