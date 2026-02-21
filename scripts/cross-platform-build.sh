#!/bin/bash
#
# cross-platform-build.sh - Interactive cross-platform build script for Tauri 2.x
#
# Supports building for:
#   - Linux: x86_64, i686 (32-bit), aarch64 (ARM64), armv7 (ARM32)
#   - Windows: x86_64, i686 (32-bit), x86_64-gnull (GNU toolchain)
#   - macOS: x86_64 (Intel), aarch64 (Apple Silicon), universal
#
# Usage:
#   ./scripts/cross-platform-build.sh
#   TARGETS="linux-x86_64,windows-x86_64" ./scripts/cross-platform-build.sh
#   ALL_TARGETS=1 ./scripts/cross-platform-build.sh
#

set -e

# ---------------------------------------------------------------------------
# Colors and formatting
# ---------------------------------------------------------------------------
readonly RED='\033[0;31m'
readonly GREEN='\033[0;32m'
readonly YELLOW='\033[1;33m'
readonly BLUE='\033[0;34m'
readonly CYAN='\033[0;36m'
readonly BOLD='\033[1m'
readonly NC='\033[0m' # No Color

# ---------------------------------------------------------------------------
# Script directory and project root
# ---------------------------------------------------------------------------
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
SRC_TAURI_DIR="$PROJECT_ROOT/src-tauri"

# ---------------------------------------------------------------------------
# Build configuration
# ---------------------------------------------------------------------------
# All available targets
declare -A LINUX_TARGETS=(
    ["linux-x86_64"]="x86_64-unknown-linux-gnu"
    ["linux-i686"]="i686-unknown-linux-gnu"
    ["linux-aarch64"]="aarch64-unknown-linux-gnu"
    ["linux-armv7"]="armv7-unknown-linux-gnueabihf"
)

declare -A WINDOWS_TARGETS=(
    ["windows-x86_64"]="x86_64-pc-windows-msvc"
    ["windows-i686"]="i686-pc-windows-msvc"
    ["windows-x86_64-gnu"]="x86_64-pc-windows-gnu"
)

declare -A MACOS_TARGETS=(
    ["macos-x86_64"]="x86_64-apple-darwin"
    ["macos-aarch64"]="aarch64-apple-darwin"
    ["macos-universal"]="universal-apple-darwin"
)

# Bundle types for each platform
LINUX_BUNDLES="appimage,deb,rpm"
WINDOWS_BUNDLES="msi,nsis"
MACOS_BUNDLES="dmg,app"

# Selected targets (can be set via environment)
SELECTED_TARGETS=()

# Build options
CLEAN_BUILD=0
VERBOSE=0
SKIP_FRONTEND=0
SIGN_BUILD=0
RELEASE_PROFILE="release"

# ---------------------------------------------------------------------------
# Helper functions
# ---------------------------------------------------------------------------

log_info() {
    echo -e "${BLUE}[INFO]${NC} $*"
}

log_success() {
    echo -e "${GREEN}[SUCCESS]${NC} $*"
}

log_warning() {
    echo -e "${YELLOW}[WARNING]${NC} $*"
}

log_error() {
    echo -e "${RED}[ERROR]${NC} $*"
}

log_step() {
    echo -e "${CYAN}${BOLD}==>${NC} ${BOLD}$*${NC}"
}

print_header() {
    echo ""
    echo -e "${BOLD}${CYAN}╔════════════════════════════════════════════════════════════╗${NC}"
    echo -e "${BOLD}${CYAN}║${NC}  ${BOLD}$1${NC}"
    echo -e "${BOLD}${CYAN}╚════════════════════════════════════════════════════════════╝${NC}"
    echo ""
}

# ---------------------------------------------------------------------------
# Detect host platform
# ---------------------------------------------------------------------------
detect_host_platform() {
    local os="$(uname -s)"
    local arch="$(uname -m)"

    case "$os" in
        Linux)
            HOST_OS="linux"
            ;;
        Darwin)
            HOST_OS="macos"
            ;;
        MINGW*|MSYS*|CYGWIN*)
            HOST_OS="windows"
            ;;
        *)
            log_error "Unknown host OS: $os"
            exit 1
            ;;
    esac

    case "$arch" in
        x86_64|amd64)
            HOST_ARCH="x86_64"
            ;;
        i686|i386)
            HOST_ARCH="i686"
            ;;
        aarch64|arm64)
            HOST_ARCH="aarch64"
            ;;
        armv7l)
            HOST_ARCH="armv7"
            ;;
        *)
            log_warning "Unknown host architecture: $arch"
            HOST_ARCH="$arch"
            ;;
    esac

    log_info "Detected host platform: $HOST_OS-$HOST_ARCH"
}

# ---------------------------------------------------------------------------
# Check prerequisites
# ---------------------------------------------------------------------------
check_prerequisites() {
    log_step "Checking prerequisites..."

    local missing_deps=()

    # Check for required commands
    local required_cmds=("cargo" "rustc" "bun" "node")
    for cmd in "${required_cmds[@]}"; do
        if ! command -v "$cmd" &> /dev/null; then
            missing_deps+=("$cmd")
        fi
    done

    # Platform-specific checks
    if [[ "$HOST_OS" == "linux" ]]; then
        # Check for cross-compilation tools
        local has_linux_i686=0
        local has_linux_aarch64=0
        local has_linux_armv7=0
        for t in "${SELECTED_TARGETS[@]}"; do
            case "$t" in
                linux-i686) has_linux_i686=1 ;;
                linux-aarch64) has_linux_aarch64=1 ;;
                linux-armv7) has_linux_armv7=1 ;;
            esac
        done

        if [[ $has_linux_i686 -eq 1 ]] && ! command -v gcc-multilib &> /dev/null; then
            missing_deps+=("gcc-multilib (for 32-bit Linux builds)")
        fi
        if [[ $has_linux_aarch64 -eq 1 ]] && ! command -v aarch64-linux-gnu-gcc &> /dev/null; then
            missing_deps+=("gcc-aarch64-linux-gnu (for ARM64 Linux builds)")
        fi
        if [[ $has_linux_armv7 -eq 1 ]] && ! command -v arm-linux-gnueabihf-gcc &> /dev/null; then
            missing_deps+=("gcc-arm-linux-gnueabihf (for ARMv7 Linux builds)")
        fi
    fi

    # Check if Rust targets are installed
    for target in "${SELECTED_TARGETS[@]}"; do
        if ! rustup target list --installed 2>/dev/null | grep -q "$target"; then
            log_warning "Rust target '$target' not installed. Will attempt to install."
        fi
    done

    if [[ ${#missing_deps[@]} -gt 0 ]]; then
        log_error "Missing required dependencies:"
        for dep in "${missing_deps[@]}"; do
            echo "  - $dep"
        done
        echo ""
        log_info "Install commands:"
        if [[ "$HOST_OS" == "linux" ]]; then
            echo "  sudo apt-get install cargo rustc bun nodejs gcc-multilib gcc-aarch64-linux-gnu gcc-arm-linux-gnueabihf"
        elif [[ "$HOST_OS" == "macos" ]]; then
            echo "  brew install rust bun node"
        fi
        exit 1
    fi

    log_success "All prerequisites met"
}

# ---------------------------------------------------------------------------
# Install Rust targets
# ---------------------------------------------------------------------------
install_rust_targets() {
    log_step "Installing Rust targets..."

    for target in "${SELECTED_TARGETS[@]}"; do
        if rustup target list --installed 2>/dev/null | grep -q "$target"; then
            log_info "Target '$target' already installed"
        else
            log_info "Installing target '$target'..."
            rustup target add "$target" || log_warning "Failed to install target '$target'"
        fi
    done
}

# ---------------------------------------------------------------------------
# Build frontend
# ---------------------------------------------------------------------------
build_frontend() {
    if [[ "$SKIP_FRONTEND" -eq 1 ]]; then
        log_info "Skipping frontend build"
        return
    fi

    log_step "Building frontend..."
    cd "$PROJECT_ROOT"

    if [[ ! -d "node_modules" ]]; then
        log_info "Installing frontend dependencies..."
        bun install --frozen-lockfile
    fi

    log_info "Building frontend with bun..."
    bun run build

    log_success "Frontend build complete"
}

# ---------------------------------------------------------------------------
# Build Tauri application
# ---------------------------------------------------------------------------
build_tauri() {
    local target="$1"
    local bundles="$2"

    log_step "Building Tauri application for $target..."
    log_info "Bundle types: $bundles"

    cd "$PROJECT_ROOT"

    local tauri_cmd="tauri build"

    if [[ -n "$target" ]]; then
        tauri_cmd="$tauri_cmd --target $target"
    fi

    if [[ -n "$bundles" ]]; then
        tauri_cmd="$tauri_cmd --bundles $bundles"
    fi

    if [[ "$CLEAN_BUILD" -eq 1 ]]; then
        log_info "Cleaning previous build artifacts..."
        cargo clean --manifest-path="$SRC_TAURI_DIR/Cargo.toml"
    fi

    if [[ "$VERBOSE" -eq 1 ]]; then
        tauri_cmd="$tauri_cmd --verbose"
    fi

    log_info "Running: $tauri_cmd"
    eval "$tauri_cmd"

    log_success "Build complete for $target"
}

# ---------------------------------------------------------------------------
# Build for Linux
# ---------------------------------------------------------------------------
build_linux() {
    log_step "Building for Linux..."

    if [[ "$HOST_OS" != "linux" ]]; then
        log_error "Linux builds must be performed on Linux"
        log_info "Use Docker or GitHub Actions for cross-compilation"
        return 1
    fi

    # Install system dependencies if needed
    if ! dpkg -l | grep -q libwebkit2gtk-4.1-dev; then
        log_info "Installing Linux build dependencies..."
        sudo apt-get update
        sudo apt-get install -y \
            libwebkit2gtk-4.1-dev \
            libappindicator3-dev \
            librsvg2-dev \
            patchelf \
            libasound2-dev \
            libgtk-3-dev
    fi

    for target_name in "${!LINUX_TARGETS[@]}"; do
        if [[ " ${SELECTED_TARGETS[*]} " =~ " ${target_name} " ]]; then
            local triple="${LINUX_TARGETS[$target_name]}"
            build_tauri "$triple" "$LINUX_BUNDLES"
        fi
    done
}

# ---------------------------------------------------------------------------
# Build for Windows (cross-compile from Linux)
# ---------------------------------------------------------------------------
build_windows() {
    log_step "Building for Windows..."

    if [[ "$HOST_OS" == "windows" ]]; then
        # Native Windows build
        for target_name in "${!WINDOWS_TARGETS[@]}"; do
            if [[ " ${SELECTED_TARGETS[*]} " =~ " ${target_name} " ]]; then
                local triple="${WINDOWS_TARGETS[$target_name]}"
                build_tauri "$triple" "$WINDOWS_BUNDLES"
            fi
        done
    elif [[ "$HOST_OS" == "linux" ]]; then
        # Cross-compile from Linux using MinGW
        log_info "Setting up MinGW cross-compilation..."

        # Install MinGW toolchain
        if ! dpkg -l | grep -q mingw-w64; then
            sudo apt-get install -y mingw-w64
        fi

        # Add Windows toolchain to Rust
        cat > "$SRC_TAURI_DIR/.cargo/config.toml" << 'EOF'
[target.x86_64-pc-windows-gnu]
ar = "x86_64-w64-mingw32-gcc-ar"
linker = "x86_64-w64-mingw32-gcc"

[target.i686-pc-windows-gnu]
ar = "i686-w64-mingw32-gcc-ar"
linker = "i686-w64-mingw32-gcc"
EOF

        for target_name in "${!WINDOWS_TARGETS[@]}"; do
            if [[ " ${SELECTED_TARGETS[*]} " =~ " ${target_name} " ]]; then
                # Only build GNU targets from Linux (MSVC requires Windows)
                if [[ "$target_name" == *"gnu"* ]]; then
                    local triple="${WINDOWS_TARGETS[$target_name]}"
                    build_tauri "$triple" "nsis"  # NSIS works with GNU toolchain
                else
                    log_warning "MSVC builds require Windows host. Skipping $target_name"
                fi
            fi
        done
    else
        log_error "Windows builds require Windows or Linux host"
        return 1
    fi
}

# ---------------------------------------------------------------------------
# Build for macOS
# ---------------------------------------------------------------------------
build_macos() {
    log_step "Building for macOS..."

    if [[ "$HOST_OS" != "macos" ]]; then
        log_error "macOS builds must be performed on macOS"
        log_info "Use GitHub Actions or MacStadium for cross-compilation"
        return 1
    fi

    # Check for Xcode
    if ! command -v xcodebuild &> /dev/null; then
        log_error "Xcode command line tools not found"
        log_info "Install with: xcode-select --install"
        return 1
    fi

    for target_name in "${!MACOS_TARGETS[@]}"; do
        if [[ " ${SELECTED_TARGETS[*]} " =~ " ${target_name} " ]]; then
            local triple="${MACOS_TARGETS[$target_name]}"
            build_tauri "$triple" "$MACOS_BUNDLES"
        fi
    done
}

# ---------------------------------------------------------------------------
# Display build artifacts
# ---------------------------------------------------------------------------
display_artifacts() {
    log_step "Build Artifacts"
    echo ""

    local artifact_dir="$SRC_TAURI_DIR/target/release/bundle"
    if [[ -d "$artifact_dir" ]]; then
        log_info "Generated files:"
        echo ""

        # Linux
        if [[ -d "$artifact_dir/appimage" ]]; then
            echo -e "${GREEN}Linux (AppImage):${NC}"
            ls -lh "$artifact_dir/appimage"/*.AppImage 2>/dev/null || echo "  No AppImage files found"
            echo ""
        fi
        if [[ -d "$artifact_dir/deb" ]]; then
            echo -e "${GREEN}Linux (Debian):${NC}"
            ls -lh "$artifact_dir/deb"/*.deb 2>/dev/null || echo "  No deb files found"
            echo ""
        fi
        if [[ -d "$artifact_dir/rpm" ]]; then
            echo -e "${GREEN}Linux (RPM):${NC}"
            ls -lh "$artifact_dir/rpm"/*.rpm 2>/dev/null || echo "  No rpm files found"
            echo ""
        fi

        # Windows
        if [[ -d "$artifact_dir/msi" ]]; then
            echo -e "${GREEN}Windows (MSI):${NC}"
            ls -lh "$artifact_dir/msi"/*.msi 2>/dev/null || echo "  No msi files found"
            echo ""
        fi
        if [[ -d "$artifact_dir/nsis" ]]; then
            echo -e "${GREEN}Windows (NSIS):${NC}"
            ls -lh "$artifact_dir/nsis"/*.exe 2>/dev/null || echo "  No exe files found"
            echo ""
        fi

        # macOS
        if [[ -d "$artifact_dir/macos" ]]; then
            echo -e "${GREEN}macOS (DMG):${NC}"
            ls -lh "$artifact_dir/macos"/*.dmg 2>/dev/null || echo "  No dmg files found"
            echo ""
        fi
        if [[ -d "$artifact_dir/dmg" ]]; then
            echo -e "${GREEN}macOS (DMG - alt):${NC}"
            ls -lh "$artifact_dir/dmg"/*.dmg 2>/dev/null || echo "  No dmg files found"
            echo ""
        fi

        # Universal binaries
        if [[ -d "$SRC_TAURI_DIR/target/universal-apple-darwin/release" ]]; then
            echo -e "${GREEN}macOS (Universal binary):${NC}"
            ls -lh "$SRC_TAURI_DIR/target/universal-apple-darwin/release/" 2>/dev/null
            echo ""
        fi
    fi
}

# ---------------------------------------------------------------------------
# Interactive menu
# ---------------------------------------------------------------------------
show_menu() {
    print_header "Tauri 2.x Cross-Platform Build Script"

    echo -e "${BOLD}Select target platforms:${NC}"
    echo ""
    echo "  ${CYAN}Linux${NC}"
    echo "    1) x86_64 (64-bit Intel/AMD)"
    echo "    2) i686 (32-bit)"
    echo "    3) aarch64 (ARM64)"
    echo "    4) armv7 (ARM32)"
    echo "    5) All Linux targets"
    echo ""
    echo "  ${CYAN}Windows${NC}"
    echo "    6) x86_64 (64-bit MSVC)"
    echo "    7) i686 (32-bit MSVC)"
    echo "    8) x86_64-gnu (64-bit MinGW)"
    echo "    9) All Windows targets"
    echo ""
    echo "  ${CYAN}macOS${NC}"
    echo "    a) x86_64 (Intel)"
    echo "    b) aarch64 (Apple Silicon)"
    echo "    c) universal (Intel + Silicon)"
    echo "    d) All macOS targets"
    echo ""
    echo "  ${CYAN}Presets${NC}"
    echo "    L) Linux + Windows + macOS (All)"
    echo "    D) Default (Linux x86_64, Windows x86_64, macOS universal)"
    echo ""
    echo "  ${CYAN}Options${NC}"
    echo "    C) Toggle clean build (currently: $CLEAN_BUILD)"
    echo "    V) Toggle verbose output (currently: $VERBOSE)"
    echo "    S) Toggle skip frontend (currently: $SKIP_FRONTEND)"
    echo ""
    echo "  ${GREEN}B) Build${NC}"
    echo "  ${RED}Q) Quit${NC}"
    echo ""
}

# ---------------------------------------------------------------------------
# Parse command line arguments
# ---------------------------------------------------------------------------
parse_arguments() {
    while [[ $# -gt 0 ]]; do
        case "$1" in
            --targets|-t)
                IFS=',' read -ra TARGETS <<< "$2"
                for t in "${TARGETS[@]}"; do
                    t=$(echo "$t" | tr -d ' ')
                    SELECTED_TARGETS+=("$t")
                done
                shift 2
                ;;
            --all)
                SELECTED_TARGETS=(
                    "linux-x86_64" "linux-i686" "linux-aarch64" "linux-armv7"
                    "windows-x86_64" "windows-i686" "windows-x86_64-gnu"
                    "macos-x86_64" "macos-aarch64" "macos-universal"
                )
                shift
                ;;
            --linux)
                SELECTED_TARGETS+=("linux-x86_64" "linux-i686" "linux-aarch64" "linux-armv7")
                shift
                ;;
            --windows)
                SELECTED_TARGETS+=("windows-x86_64" "windows-i686" "windows-x86_64-gnu")
                shift
                ;;
            --macos)
                SELECTED_TARGETS+=("macos-x86_64" "macos-aarch64" "macos-universal")
                shift
                ;;
            --default)
                SELECTED_TARGETS=("linux-x86_64" "windows-x86_64" "macos-universal")
                shift
                ;;
            --clean)
                CLEAN_BUILD=1
                shift
                ;;
            --verbose|-v)
                VERBOSE=1
                shift
                ;;
            --skip-frontend)
                SKIP_FRONTEND=1
                shift
                ;;
            --help|-h)
                show_help
                exit 0
                ;;
            *)
                log_error "Unknown option: $1"
                show_help
                exit 1
                ;;
        esac
    done
}

show_help() {
    cat << EOF
Usage: $0 [OPTIONS]

Interactive cross-platform build script for Tauri 2.x applications.

Options:
  -t, --targets TARGETS    Comma-separated list of targets
                           (e.g., "linux-x86_64,windows-x86_64,macos-universal")
  --all                    Build for all available targets
  --linux                  Build for all Linux targets
  --windows                Build for all Windows targets
  --macos                  Build for all macOS targets
  --default                Build default targets (Linux x86_64, Windows x86_64, macOS universal)
  --clean                  Clean build artifacts before building
  --verbose, -v            Enable verbose output
  --skip-frontend          Skip frontend build
  --help, -h               Show this help message

Available Targets:
  Linux:
    linux-x86_64           x86_64-unknown-linux-gnu (64-bit)
    linux-i686             i686-unknown-linux-gnu (32-bit)
    linux-aarch64          aarch64-unknown-linux-gnu (ARM64)
    linux-armv7            armv7-unknown-linux-gnueabihf (ARM32)

  Windows:
    windows-x86_64         x86_64-pc-windows-msvc (64-bit MSVC)
    windows-i686           i686-pc-windows-msvc (32-bit MSVC)
    windows-x86_64-gnu     x86_64-pc-windows-gnu (64-bit MinGW)

  macOS:
    macos-x86_64           x86_64-apple-darwin (Intel)
    macos-aarch64          aarch64-apple-darwin (Apple Silicon)
    macos-universal        universal-apple-darwin (Universal)

Environment Variables:
  TARGETS                  Same as --targets
  ALL_TARGETS=1            Same as --all
  CLEAN_BUILD=1            Same as --clean
  VERBOSE=1                Same as --verbose
  SKIP_FRONTEND=1          Same as --skip-frontend

Examples:
  # Interactive mode
  $0

  # Build for specific targets
  $0 --targets linux-x86_64,windows-x86_64

  # Build default targets with clean build
  $0 --default --clean

  # Build all targets
  $0 --all

  # Using environment variables
  TARGETS="linux-x86_64,macos-universal" $0

EOF
}

# ---------------------------------------------------------------------------
# Interactive selection loop
# ---------------------------------------------------------------------------
interactive_selection() {
    while true; do
        clear
        show_menu

        if [[ ${#SELECTED_TARGETS[@]} -gt 0 ]]; then
            echo -e "${BOLD}Selected targets:${NC}"
            for t in "${SELECTED_TARGETS[@]}"; do
                echo "  - ${GREEN}$t${NC}"
            done
            echo ""
        fi

        read -p "Enter choice: " -n 1 -r choice
        echo ""

        case "$choice" in
            1) SELECTED_TARGETS+=("linux-x86_64") ;;
            2) SELECTED_TARGETS+=("linux-i686") ;;
            3) SELECTED_TARGETS+=("linux-aarch64") ;;
            4) SELECTED_TARGETS+=("linux-armv7") ;;
            5)
                SELECTED_TARGETS+=("linux-x86_64" "linux-i686" "linux-aarch64" "linux-armv7")
                ;;
            6) SELECTED_TARGETS+=("windows-x86_64") ;;
            7) SELECTED_TARGETS+=("windows-i686") ;;
            8) SELECTED_TARGETS+=("windows-x86_64-gnu") ;;
            9)
                SELECTED_TARGETS+=("windows-x86_64" "windows-i686" "windows-x86_64-gnu")
                ;;
            a|A) SELECTED_TARGETS+=("macos-x86_64") ;;
            b|B) SELECTED_TARGETS+=("macos-aarch64") ;;
            c|C) SELECTED_TARGETS+=("macos-universal") ;;
            d|D)
                SELECTED_TARGETS+=("macos-x86_64" "macos-aarch64" "macos-universal")
                ;;
            l|L)
                SELECTED_TARGETS=(
                    "linux-x86_64" "linux-i686" "linux-aarch64" "linux-armv7"
                    "windows-x86_64" "windows-i686" "windows-x86_64-gnu"
                    "macos-x86_64" "macos-aarch64" "macos-universal"
                )
                ;;
            # Preset: Default
            D) SELECTED_TARGETS=("linux-x86_64" "windows-x86_64" "macos-universal") ;;

            # Options
            c|C)
                if [[ "$CLEAN_BUILD" -eq 1 ]]; then
                    CLEAN_BUILD=0
                else
                    CLEAN_BUILD=1
                fi
                ;;
            v|V)
                if [[ "$VERBOSE" -eq 1 ]]; then
                    VERBOSE=0
                else
                    VERBOSE=1
                fi
                ;;
            s|S)
                if [[ "$SKIP_FRONTEND" -eq 1 ]]; then
                    SKIP_FRONTEND=0
                else
                    SKIP_FRONTEND=1
                fi
                ;;

            # Build
            b|B)
                if [[ ${#SELECTED_TARGETS[@]} -eq 0 ]]; then
                    log_error "No targets selected. Please select at least one target."
                    read -p "Press Enter to continue..."
                    continue
                fi
                break
                ;;

            # Quit
            q|Q)
                log_info "Build cancelled."
                exit 0
                ;;

            *)
                log_error "Invalid choice: $choice"
                read -p "Press Enter to continue..."
                ;;
        esac
    done
}

# ---------------------------------------------------------------------------
# Main build execution
# ---------------------------------------------------------------------------
main() {
    # Parse environment variables for default values
    if [[ -n "$TARGETS" ]]; then
        IFS=',' read -ra TARGETS <<< "$TARGETS"
        for t in "${TARGETS[@]}"; do
            t=$(echo "$t" | tr -d ' ')
            SELECTED_TARGETS+=("$t")
        done
    fi

    if [[ "$ALL_TARGETS" == "1" ]]; then
        SELECTED_TARGETS=(
            "linux-x86_64" "linux-i686" "linux-aarch64" "linux-armv7"
            "windows-x86_64" "windows-i686" "windows-x86_64-gnu"
            "macos-x86_64" "macos-aarch64" "macos-universal"
        )
    fi

    [[ "$CLEAN_BUILD" == "1" ]] && CLEAN_BUILD=1
    [[ "$VERBOSE" == "1" ]] && VERBOSE=1
    [[ "$SKIP_FRONTEND" == "1" ]] && SKIP_FRONTEND=1

    # Parse command line arguments
    parse_arguments "$@"

    # Detect host platform
    detect_host_platform

    # If no targets selected, run interactive mode
    if [[ ${#SELECTED_TARGETS[@]} -eq 0 ]]; then
        interactive_selection
    fi

    # Convert target names to Rust triples
    declare -a RUST_TRIPLES=()
    declare -a PLATFORMS=()

    for target in "${SELECTED_TARGETS[@]}"; do
        if [[ -n "${LINUX_TARGETS[$target]}" ]]; then
            RUST_TRIPLES+=("${LINUX_TARGETS[$target]}")
            PLATFORMS+=("linux")
        elif [[ -n "${WINDOWS_TARGETS[$target]}" ]]; then
            RUST_TRIPLES+=("${WINDOWS_TARGETS[$target]}")
            PLATFORMS+=("windows")
        elif [[ -n "${MACOS_TARGETS[$target]}" ]]; then
            RUST_TRIPLES+=("${MACOS_TARGETS[$target]}")
            PLATFORMS+=("macos")
        fi
    done

    print_header "Starting Build"
    log_info "Host: $HOST_OS-$HOST_ARCH"
    log_info "Selected targets: ${SELECTED_TARGETS[*]}"
    log_info "Clean build: $CLEAN_BUILD"
    log_info "Verbose: $VERBOSE"
    echo ""

    # Check prerequisites
    check_prerequisites

    # Install Rust targets
    install_rust_targets

    # Build frontend
    build_frontend

    # Build for each platform
    for platform in "${PLATFORMS[@]}"; do
        case "$platform" in
            linux)
                build_linux
                ;;
            windows)
                build_windows
                ;;
            macos)
                build_macos
                ;;
        esac
    done

    # Display artifacts
    display_artifacts

    print_header "Build Complete!"
    log_success "All builds finished successfully!"
}

# Run main
main "$@"
