#!/bin/bash
set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# File paths (relative to project root)
PACKAGE_JSON="package.json"
CARGO_TOML="src-tauri/Cargo.toml"
TAURI_CONF="src-tauri/tauri.conf.json"

# Get script directory and move to project root
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$SCRIPT_DIR/.."

#######################################
# Print colored message
#######################################
print_msg() {
    local color=$1
    local msg=$2
    echo -e "${color}${msg}${NC}"
}

#######################################
# Get current version from package.json
#######################################
get_current_version() {
    grep -o '"version": "[^"]*"' "$PACKAGE_JSON" | head -1 | cut -d'"' -f4
}

#######################################
# Increment version based on type
# Arguments: current_version bump_type
#######################################
increment_version() {
    local version=$1
    local bump_type=$2

    IFS='.' read -r major minor patch <<< "$version"

    case $bump_type in
        major)
            major=$((major + 1))
            minor=0
            patch=0
            ;;
        minor)
            minor=$((minor + 1))
            patch=0
            ;;
        patch)
            patch=$((patch + 1))
            ;;
        *)
            print_msg "$RED" "Invalid bump type: $bump_type"
            exit 1
            ;;
    esac

    echo "$major.$minor.$patch"
}

#######################################
# Update version in package.json
#######################################
update_package_json() {
    local version=$1
    if [[ "$OSTYPE" == "darwin"* ]]; then
        sed -i '' "s/\"version\": \"[^\"]*\"/\"version\": \"$version\"/" "$PACKAGE_JSON"
    else
        sed -i "s/\"version\": \"[^\"]*\"/\"version\": \"$version\"/" "$PACKAGE_JSON"
    fi
    print_msg "$GREEN" "✓ Updated $PACKAGE_JSON to $version"
}

#######################################
# Update version in Cargo.toml
#######################################
update_cargo_toml() {
    local version=$1
    if [[ "$OSTYPE" == "darwin"* ]]; then
        sed -i '' "s/^version = \"[^\"]*\"/version = \"$version\"/" "$CARGO_TOML"
    else
        sed -i "s/^version = \"[^\"]*\"/version = \"$version\"/" "$CARGO_TOML"
    fi
    print_msg "$GREEN" "✓ Updated $CARGO_TOML to $version"
}

#######################################
# Update version in tauri.conf.json
#######################################
update_tauri_conf() {
    local version=$1
    if [[ "$OSTYPE" == "darwin"* ]]; then
        sed -i '' "s/\"version\": \"[^\"]*\"/\"version\": \"$version\"/" "$TAURI_CONF"
    else
        sed -i "s/\"version\": \"[^\"]*\"/\"version\": \"$version\"/" "$TAURI_CONF"
    fi
    print_msg "$GREEN" "✓ Updated $TAURI_CONF to $version"
}

#######################################
# Sync all files to specified version
#######################################
sync_versions() {
    local version=$1
    update_package_json "$version"
    update_cargo_toml "$version"
    update_tauri_conf "$version"
}

#######################################
# Collect commit messages since last tag
#######################################
get_changelog() {
    local last_tag
    last_tag=$(git describe --tags --abbrev=0 2>/dev/null || echo "")

    if [[ -n "$last_tag" ]]; then
        git log "$last_tag"..HEAD --oneline --no-merges
    else
        git log --oneline --no-merges -10
    fi
}

#######################################
# Create release commit and push
#######################################
create_release() {
    local version=$1
    local changelog

    # Get changelog
    changelog=$(get_changelog)

    # Stage version files
    git add "$PACKAGE_JSON" "$CARGO_TOML" "$TAURI_CONF"

    # Create commit message with changelog
    local commit_msg="chore(release): v$version

## Changes in this release:
$changelog

---
Automated release by scripts/release.sh"

    git commit -m "$commit_msg"
    print_msg "$GREEN" "✓ Created release commit"

    # Create tag
    git tag -a "app-v$version" -m "Release v$version"
    print_msg "$GREEN" "✓ Created tag app-v$version"
}

#######################################
# Push to release branch (triggers CI)
#######################################
push_release() {
    local current_branch
    current_branch=$(git branch --show-current)

    # Push commit and tag
    git push origin "$current_branch"
    git push origin --tags
    print_msg "$GREEN" "✓ Pushed to $current_branch with tags"

    # If not on release branch, offer to push there
    if [[ "$current_branch" != "release" ]]; then
        print_msg "$YELLOW" ""
        print_msg "$YELLOW" "You're on '$current_branch', not 'release'."
        print_msg "$YELLOW" "To trigger the build, merge to release or push directly:"
        print_msg "$BLUE" "  git push origin $current_branch:release"
        echo ""
        read -p "Push to release branch now? (y/N) " -n 1 -r
        echo
        if [[ $REPLY =~ ^[Yy]$ ]]; then
            git push origin "$current_branch:release"
            print_msg "$GREEN" "✓ Pushed to release branch - GitHub Actions will start building!"
        fi
    else
        print_msg "$GREEN" "✓ Pushed to release branch - GitHub Actions will start building!"
    fi
}

#######################################
# Show usage
#######################################
usage() {
    echo ""
    echo "Usage: $0 <command> [version]"
    echo ""
    echo "Commands:"
    echo "  patch      Bump patch version (0.0.1 → 0.0.2)"
    echo "  minor      Bump minor version (0.0.1 → 0.1.0)"
    echo "  major      Bump major version (0.0.1 → 1.0.0)"
    echo "  sync       Sync all files to current package.json version"
    echo "  set <ver>  Set specific version (e.g., set 1.2.3)"
    echo "  status     Show current versions in all files"
    echo ""
    echo "Examples:"
    echo "  $0 patch           # Bump patch and release"
    echo "  $0 minor           # Bump minor and release"
    echo "  $0 set 0.0.1       # Set specific version"
    echo "  $0 status          # Check version sync status"
    echo ""
}

#######################################
# Show version status across all files
#######################################
show_status() {
    echo ""
    print_msg "$BLUE" "Version Status:"
    echo "─────────────────────────────────────"

    local pkg_ver cargo_ver tauri_ver
    pkg_ver=$(grep -o '"version": "[^"]*"' "$PACKAGE_JSON" | head -1 | cut -d'"' -f4)
    cargo_ver=$(grep '^version = ' "$CARGO_TOML" | head -1 | cut -d'"' -f2)
    tauri_ver=$(grep -o '"version": "[^"]*"' "$TAURI_CONF" | head -1 | cut -d'"' -f4)

    echo "  package.json:      $pkg_ver"
    echo "  Cargo.toml:        $cargo_ver"
    echo "  tauri.conf.json:   $tauri_ver"
    echo ""

    if [[ "$pkg_ver" == "$cargo_ver" && "$cargo_ver" == "$tauri_ver" ]]; then
        print_msg "$GREEN" "✓ All versions are in sync!"
    else
        print_msg "$RED" "✗ Versions are out of sync!"
        print_msg "$YELLOW" "  Run: $0 sync"
    fi
    echo ""
}

#######################################
# Main
#######################################
main() {
    local command=${1:-""}
    local arg=${2:-""}

    case $command in
        patch|minor|major)
            local current_version new_version
            current_version=$(get_current_version)
            new_version=$(increment_version "$current_version" "$command")

            print_msg "$BLUE" ""
            print_msg "$BLUE" "Releasing v$new_version ($command bump from $current_version)"
            print_msg "$BLUE" "─────────────────────────────────────"

            sync_versions "$new_version"
            create_release "$new_version"
            push_release

            print_msg "$GREEN" ""
            print_msg "$GREEN" "🚀 Release v$new_version complete!"
            ;;

        sync)
            local current_version
            current_version=$(get_current_version)
            print_msg "$BLUE" "Syncing all files to v$current_version"
            sync_versions "$current_version"
            ;;

        set)
            if [[ -z "$arg" ]]; then
                print_msg "$RED" "Error: Please provide a version number"
                echo "  Example: $0 set 1.2.3"
                exit 1
            fi

            # Validate semver format
            if ! [[ "$arg" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]]; then
                print_msg "$RED" "Error: Invalid version format. Use semver (e.g., 1.2.3)"
                exit 1
            fi

            print_msg "$BLUE" "Setting version to v$arg"
            sync_versions "$arg"
            ;;

        status)
            show_status
            ;;

        *)
            usage
            exit 1
            ;;
    esac
}

main "$@"
