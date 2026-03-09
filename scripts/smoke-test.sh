#!/usr/bin/env bash
# MesoClaw -- Binary Smoke Test Script
# Usage: ./scripts/smoke-test.sh <BINARY_PATH> [OPTIONS]
#
# Arguments:
#   <BINARY_PATH>         Path to the binary to test
#
# Options:
#   --expected-arch <ARCH>  Expected architecture string from `file` command
#                           (e.g., "x86-64", "aarch64", "ARM", "PE32+")
#   --max-size <SIZE_MB>    Maximum acceptable binary size in MB (default: 100)
#   --help                  Show this help message

set -euo pipefail

# ── Colors ─────────────────────────────────────────────────────────────
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
CYAN='\033[0;36m'
NC='\033[0m'

info()  { echo -e "${BLUE}[INFO]${NC}  $*"; }
ok()    { echo -e "${GREEN}[PASS]${NC}  $*"; }
warn()  { echo -e "${YELLOW}[WARN]${NC}  $*"; }
fail()  { echo -e "${RED}[FAIL]${NC}  $*"; FAILURES=$((FAILURES + 1)); }
step()  { echo -e "${CYAN}[TEST]${NC}  $*"; }

# ── Defaults ───────────────────────────────────────────────────────────
BINARY_PATH=""
EXPECTED_ARCH=""
MAX_SIZE_MB=100
FAILURES=0

# ── Functions ──────────────────────────────────────────────────────────

show_help() {
    head -11 "$0" | tail -10 | sed 's/^# //' | sed 's/^#//'
}

check_exists() {
    step "Checking binary exists..."
    if [ ! -f "$BINARY_PATH" ]; then
        fail "Binary not found: $BINARY_PATH"
        return 1
    fi
    ok "Binary exists: $BINARY_PATH"
}

check_executable() {
    step "Checking binary is executable..."
    if [ ! -x "$BINARY_PATH" ]; then
        fail "Binary is not executable: $BINARY_PATH"
        info "  Fix with: chmod +x $BINARY_PATH"
        return 1
    fi
    ok "Binary is executable"
}

check_version() {
    step "Checking --version flag..."
    local version_output
    if version_output=$("$BINARY_PATH" --version 2>&1); then
        ok "--version works: $version_output"
    else
        # Some binaries exit non-zero for --version, check if output exists
        if [ -n "$version_output" ]; then
            warn "--version exited non-zero but produced output: $version_output"
        else
            fail "--version failed with no output"
            return 1
        fi
    fi
}

check_help() {
    step "Checking --help flag..."
    local help_output
    if help_output=$("$BINARY_PATH" --help 2>&1); then
        local line_count
        line_count=$(echo "$help_output" | wc -l)
        ok "--help works ($line_count lines of output)"
    else
        if [ -n "$help_output" ]; then
            warn "--help exited non-zero but produced output"
        else
            fail "--help failed with no output"
            return 1
        fi
    fi
}

check_size() {
    step "Checking binary size..."
    local size_bytes
    size_bytes=$(stat --format="%s" "$BINARY_PATH" 2>/dev/null || stat -f "%z" "$BINARY_PATH" 2>/dev/null)

    if [ -z "$size_bytes" ]; then
        warn "Could not determine binary size"
        return 0
    fi

    local size_mb
    size_mb=$(awk "BEGIN {printf \"%.2f\", $size_bytes / 1048576}")
    local size_human
    size_human=$(du -h "$BINARY_PATH" | awk '{print $1}')

    info "Binary size: $size_human ($size_bytes bytes)"

    local max_bytes
    max_bytes=$((MAX_SIZE_MB * 1048576))
    if [ "$size_bytes" -gt "$max_bytes" ]; then
        fail "Binary exceeds maximum size of ${MAX_SIZE_MB}MB (actual: ${size_mb}MB)"
        return 1
    fi
    ok "Binary size within limit (${size_mb}MB <= ${MAX_SIZE_MB}MB)"
}

check_arch() {
    step "Checking binary architecture..."

    if ! command -v file &> /dev/null; then
        warn "'file' command not available, skipping architecture check"
        return 0
    fi

    local file_output
    file_output=$(file "$BINARY_PATH")
    info "file output: $file_output"

    if [ -n "$EXPECTED_ARCH" ]; then
        if echo "$file_output" | grep -qi "$EXPECTED_ARCH"; then
            ok "Architecture matches expected: $EXPECTED_ARCH"
        else
            fail "Architecture mismatch! Expected '$EXPECTED_ARCH' in: $file_output"
            return 1
        fi
    else
        # Auto-detect and report
        if echo "$file_output" | grep -qi "ELF"; then
            local arch
            if echo "$file_output" | grep -qi "x86-64"; then
                arch="x86-64 (Linux)"
            elif echo "$file_output" | grep -qi "aarch64\|ARM aarch64"; then
                arch="aarch64 (Linux ARM64)"
            elif echo "$file_output" | grep -qi "ARM"; then
                arch="ARM (Linux ARMv7)"
            else
                arch="ELF (unknown arch)"
            fi
            ok "Detected architecture: $arch"
        elif echo "$file_output" | grep -qi "PE32+\|PE32"; then
            ok "Detected architecture: Windows PE"
        elif echo "$file_output" | grep -qi "Mach-O"; then
            local arch
            if echo "$file_output" | grep -qi "universal"; then
                arch="macOS Universal"
            elif echo "$file_output" | grep -qi "arm64"; then
                arch="macOS ARM64"
            elif echo "$file_output" | grep -qi "x86_64"; then
                arch="macOS x86_64"
            else
                arch="macOS (unknown)"
            fi
            ok "Detected architecture: $arch"
        else
            warn "Unknown binary format"
        fi
    fi
}

# ── Parse Arguments ────────────────────────────────────────────────────

if [ $# -eq 0 ]; then
    show_help
    exit 1
fi

# First positional argument is the binary path
BINARY_PATH="$1"
shift

while [[ $# -gt 0 ]]; do
    case "$1" in
        --expected-arch)
            EXPECTED_ARCH="$2"
            shift 2
            ;;
        --max-size)
            MAX_SIZE_MB="$2"
            shift 2
            ;;
        --help|-h)
            show_help
            exit 0
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
echo "========================================"
echo "  MesoClaw Smoke Test"
echo "  Binary: $BINARY_PATH"
if [ -n "$EXPECTED_ARCH" ]; then
echo "  Arch:   $EXPECTED_ARCH"
fi
echo "========================================"
echo ""

check_exists || true
check_executable || true

# Only run functional checks if binary exists and is executable
if [ -f "$BINARY_PATH" ] && [ -x "$BINARY_PATH" ]; then
    check_version || true
    check_help || true
fi

check_size || true
check_arch || true

echo ""
echo "========================================"
if [ "$FAILURES" -eq 0 ]; then
    ok "All smoke tests passed!"
    echo "========================================"
    exit 0
else
    fail "$FAILURES test(s) failed"
    echo "========================================"
    exit 1
fi
