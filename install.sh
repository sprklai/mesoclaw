#!/usr/bin/env bash
# Zenii -- Install Script
# Usage: curl -fsSL https://raw.githubusercontent.com/sprklai/zenii/main/install.sh | bash
#
# Options:
#   --help          Show this help message
#   --from-source   Skip binary download, build from source with cargo
#   --prefix <dir>  Install directory (default: ~/.local/bin)

set -euo pipefail

# ── Configuration ──────────────────────────────────────────────────────
REPO="sprklai/zenii"
GITHUB_API="https://api.github.com/repos/${REPO}/releases/latest"
GITHUB_DL="https://github.com/${REPO}/releases/download"
DEFAULT_PREFIX="${HOME}/.local/bin"
BINARIES="zenii zenii-daemon"

# ── Defaults ───────────────────────────────────────────────────────────
PREFIX=""
FROM_SOURCE=false

# ── Colors ─────────────────────────────────────────────────────────────
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
CYAN='\033[0;36m'
BOLD='\033[1m'
NC='\033[0m'

info()  { echo -e "${BLUE}[INFO]${NC}  $*"; }
ok()    { echo -e "${GREEN}[OK]${NC}    $*"; }
warn()  { echo -e "${YELLOW}[WARN]${NC}  $*"; }
err()   { echo -e "${RED}[ERROR]${NC} $*" >&2; }
step()  { echo -e "${CYAN}[STEP]${NC}  $*"; }

# ── Functions ──────────────────────────────────────────────────────────

show_help() {
    cat <<'EOF'
Zenii Installer

Usage:
  curl -fsSL https://raw.githubusercontent.com/sprklai/zenii/main/install.sh | bash
  bash install.sh [OPTIONS]

Options:
  --help          Show this help message
  --from-source   Skip binary download, build from source with cargo
  --prefix <dir>  Install directory (default: ~/.local/bin)

The installer will:
  1. Detect your OS and architecture
  2. Download pre-built binaries from GitHub Releases (if available)
  3. Fall back to building from source via cargo if binaries aren't found
  4. Install zenii (CLI) and zenii-daemon to the prefix directory
EOF
}

detect_platform() {
    OS="$(uname -s)"
    ARCH="$(uname -m)"

    case "$OS" in
        Linux*)  OS="linux" ;;
        Darwin*) OS="macos" ;;
        *)
            err "Unsupported operating system: $OS"
            err "Zenii supports Linux and macOS. For Windows, build from source."
            exit 1
            ;;
    esac

    case "$ARCH" in
        x86_64|amd64)   ARCH="x86_64" ;;
        aarch64|arm64)   ARCH="aarch64" ;;
        armv7l|armhf)    ARCH="armv7" ;;
        *)
            err "Unsupported architecture: $ARCH"
            exit 1
            ;;
    esac

    info "Detected platform: ${OS} ${ARCH}"
}

get_latest_version() {
    local version=""

    if command -v curl &>/dev/null; then
        version=$(curl -fsSL "$GITHUB_API" 2>/dev/null | grep '"tag_name"' | head -1 | sed 's/.*"tag_name": *"//;s/".*//')
    elif command -v wget &>/dev/null; then
        version=$(wget -qO- "$GITHUB_API" 2>/dev/null | grep '"tag_name"' | head -1 | sed 's/.*"tag_name": *"//;s/".*//')
    fi

    echo "$version"
}

download_file() {
    local url="$1"
    local dest="$2"

    if command -v curl &>/dev/null; then
        curl -fsSL -o "$dest" "$url"
    elif command -v wget &>/dev/null; then
        wget -qO "$dest" "$url"
    else
        err "Neither curl nor wget found. Cannot download."
        return 1
    fi
}

try_binary_install() {
    step "Checking for pre-built binaries..."

    local version
    version="$(get_latest_version)"

    if [ -z "$version" ]; then
        warn "Could not determine latest release version"
        return 1
    fi

    info "Latest release: ${version}"

    local tmpdir
    tmpdir="$(mktemp -d)"
    trap 'rm -rf "$tmpdir"' EXIT

    local success=true
    for bin in $BINARIES; do
        local asset="${bin}-${OS}-${ARCH}.tar.gz"
        local url="${GITHUB_DL}/${version}/${asset}"

        info "Downloading ${asset}..."
        if download_file "$url" "${tmpdir}/${asset}"; then
            if tar -xzf "${tmpdir}/${asset}" -C "$tmpdir" 2>/dev/null; then
                ok "Downloaded ${bin}"
            else
                warn "Failed to extract ${asset}"
                success=false
                break
            fi
        else
            warn "Binary not available: ${asset}"
            success=false
            break
        fi
    done

    if [ "$success" = false ]; then
        rm -rf "$tmpdir"
        trap - EXIT
        return 1
    fi

    # Install binaries to prefix
    mkdir -p "$PREFIX"
    for bin in $BINARIES; do
        if [ -f "${tmpdir}/${bin}" ]; then
            install -m 755 "${tmpdir}/${bin}" "${PREFIX}/${bin}"
            ok "Installed ${bin} -> ${PREFIX}/${bin}"
        else
            # Binary might be nested in extracted directory
            local found
            found="$(find "$tmpdir" -name "$bin" -type f 2>/dev/null | head -1)"
            if [ -n "$found" ]; then
                install -m 755 "$found" "${PREFIX}/${bin}"
                ok "Installed ${bin} -> ${PREFIX}/${bin}"
            else
                warn "Binary ${bin} not found in archive"
                success=false
            fi
        fi
    done

    rm -rf "$tmpdir"
    trap - EXIT

    if [ "$success" = false ]; then
        return 1
    fi

    return 0
}

build_from_source() {
    step "Building from source..."

    if ! command -v cargo &>/dev/null; then
        err "cargo is not installed."
        echo ""
        info "Install Rust first:"
        echo "  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh"
        echo ""
        info "Then re-run this installer."
        exit 1
    fi

    info "cargo found: $(cargo --version)"

    # Check for system dependencies on Linux
    if [ "$OS" = "linux" ]; then
        local missing=""
        if ! pkg-config --exists sqlite3 2>/dev/null; then
            missing="${missing} libsqlite3-dev"
        fi
        if ! pkg-config --exists openssl 2>/dev/null; then
            missing="${missing} libssl-dev"
        fi
        if [ -n "$missing" ]; then
            warn "Missing system dependencies:${missing}"
            echo "  Install with: sudo apt install${missing}"
            echo ""
        fi
    fi

    info "Installing zenii-cli..."
    if cargo install --git "https://github.com/${REPO}.git" zenii-cli --root "${PREFIX%/bin}" 2>&1; then
        ok "Installed zenii (CLI)"
    else
        err "Failed to build zenii-cli"
        exit 1
    fi

    info "Installing zenii-daemon..."
    if cargo install --git "https://github.com/${REPO}.git" zenii-daemon --root "${PREFIX%/bin}" 2>&1; then
        ok "Installed zenii-daemon"
    else
        err "Failed to build zenii-daemon"
        exit 1
    fi
}

check_path() {
    case ":${PATH}:" in
        *":${PREFIX}:"*)
            return 0
            ;;
    esac

    echo ""
    warn "${PREFIX} is not in your PATH."
    echo ""
    info "Add it to your shell profile:"

    local shell_name
    shell_name="$(basename "${SHELL:-/bin/bash}")"
    local rc_file
    case "$shell_name" in
        zsh)  rc_file="~/.zshrc" ;;
        fish) rc_file="~/.config/fish/config.fish" ;;
        *)    rc_file="~/.bashrc" ;;
    esac

    if [ "$shell_name" = "fish" ]; then
        echo "  echo 'set -gx PATH ${PREFIX} \$PATH' >> ${rc_file}"
    else
        echo "  echo 'export PATH=\"${PREFIX}:\$PATH\"' >> ${rc_file}"
    fi
    echo ""
    info "Then restart your shell or run:"
    echo "  export PATH=\"${PREFIX}:\$PATH\""
}

verify_install() {
    step "Verifying installation..."

    local all_ok=true
    for bin in $BINARIES; do
        if [ -x "${PREFIX}/${bin}" ]; then
            local ver
            ver="$("${PREFIX}/${bin}" --version 2>/dev/null || echo "installed")"
            ok "${bin}: ${ver}"
        else
            warn "${bin}: not found at ${PREFIX}/${bin}"
            all_ok=false
        fi
    done

    if [ "$all_ok" = true ]; then
        echo ""
        echo -e "${GREEN}${BOLD}Zenii installed successfully!${NC}"
        echo ""
        info "Quick start:"
        echo "  ${PREFIX}/zenii-daemon &    # Start the daemon"
        echo "  ${PREFIX}/zenii chat        # Start chatting"
        echo "  curl localhost:18981/health  # Check it's running"
    else
        echo ""
        warn "Some binaries could not be verified."
    fi
}

# ── Parse Arguments ────────────────────────────────────────────────────

while [ $# -gt 0 ]; do
    case "$1" in
        --help|-h)
            show_help
            exit 0
            ;;
        --from-source)
            FROM_SOURCE=true
            shift
            ;;
        --prefix)
            PREFIX="$2"
            shift 2
            ;;
        *)
            err "Unknown option: $1"
            show_help
            exit 1
            ;;
    esac
done

# ── Main ───────────────────────────────────────────────────────────────

echo ""
echo -e "${BOLD}Zenii Installer${NC}"
echo "========================================"
echo ""

detect_platform

# Resolve prefix
if [ -z "$PREFIX" ]; then
    PREFIX="$DEFAULT_PREFIX"
fi
info "Install directory: ${PREFIX}"
echo ""

if [ "$FROM_SOURCE" = true ]; then
    build_from_source
else
    if try_binary_install; then
        info "Installed from pre-built binaries"
    else
        echo ""
        info "Pre-built binaries not available for ${OS}-${ARCH}"
        info "Falling back to building from source..."
        echo ""
        build_from_source
    fi
fi

echo ""
check_path
echo ""
verify_install
echo ""
