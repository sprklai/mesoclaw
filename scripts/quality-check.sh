#!/usr/bin/env bash
# MesoClaw -- Local Quality Gate Script
# Usage: ./scripts/quality-check.sh
#
# Runs formatting, linting, tests, and banned pattern checks.
# Exit 0 only if all checks pass.

set -euo pipefail

WORKSPACE_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$WORKSPACE_ROOT"

# ── Colors ─────────────────────────────────────────────────────────────
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

info()  { echo -e "${BLUE}[INFO]${NC}  $*"; }
ok()    { echo -e "${GREEN}[PASS]${NC}  $*"; }
warn()  { echo -e "${YELLOW}[WARN]${NC}  $*"; }
fail()  { echo -e "${RED}[FAIL]${NC}  $*"; }

FAILURES=0
WARNINGS=0

run_check() {
    local name="$1"
    shift
    info "Running: $name"
    if "$@"; then
        ok "$name"
    else
        fail "$name"
        FAILURES=$((FAILURES + 1))
    fi
}

echo ""
echo "========================================"
echo "  MesoClaw Quality Gates"
echo "========================================"
echo ""

# ── 1. Formatting ─────────────────────────────────────────────────────
run_check "cargo fmt --check" cargo fmt --check --all

# ── 2. Clippy ──────────────────────────────────────────────────────────
run_check "cargo clippy" cargo clippy --workspace -- -D warnings

# ── 3. Tests ───────────────────────────────────────────────────────────
run_check "cargo test" cargo test --workspace

# ── 4. Banned Pattern Checks ──────────────────────────────────────────
info "Checking for banned patterns in non-test code..."

CORE_SRC="crates/mesoclaw-core/src"
CRATES_DIR="crates"

# Helper: search for a pattern in non-test Rust source files.
# Excludes lines inside #[cfg(test)] modules by filtering out files
# that are purely test modules, and lines after #[cfg(test)].
# Simple heuristic: grep for the pattern, exclude test files and test blocks.
check_banned_pattern() {
    local pattern="$1"
    local search_dir="$2"
    local description="$3"
    local is_error="$4"  # "error" or "warning"

    # Find matches, excluding test-related contexts
    local matches
    matches=$(grep -rn --include="*.rs" "$pattern" "$search_dir" \
        | grep -v '#\[cfg(test)\]' \
        | grep -v '#\[test\]' \
        | grep -v 'mod tests' \
        | grep -v '// test' \
        | grep -v '//.*TODO' \
        | grep -v '//.*MOCK' \
        | grep -v '_test\.rs:' \
        | grep -v 'tests/' \
        | grep -v '/test_' \
        | grep -v 'test_state' \
        | grep -v '#\[cfg(feature' \
        || true)

    if [ -n "$matches" ]; then
        if [ "$is_error" = "error" ]; then
            fail "Found banned pattern: $description"
            echo "$matches" | head -20
            if [ "$(echo "$matches" | wc -l)" -gt 20 ]; then
                echo "  ... and more (showing first 20)"
            fi
            FAILURES=$((FAILURES + 1))
        else
            warn "Found pattern (advisory): $description"
            echo "$matches" | head -10
            if [ "$(echo "$matches" | wc -l)" -gt 10 ]; then
                echo "  ... and more (showing first 10)"
            fi
            WARNINGS=$((WARNINGS + 1))
        fi
    else
        ok "No banned pattern: $description"
    fi
}

# 4a. std::sync::Mutex in core (not in test code)
check_banned_pattern 'std::sync::Mutex' "$CORE_SRC" "std::sync::Mutex in core (use tokio::sync::Mutex)" "error"

# 4b. block_on() anywhere in crates
check_banned_pattern 'block_on(' "$CRATES_DIR" "block_on() in crates (use .await)" "error"

# 4c. println!() in core (not in test code)
check_banned_pattern 'println!' "$CORE_SRC" "println!() in core (use tracing macros)" "error"

# 4d. Result<T, String> in core (not in test code)
check_banned_pattern 'Result<.*String>' "$CORE_SRC" "Result<T, String> in core (use MesoError)" "error"

# 4e. .unwrap() in non-test code (warning only)
check_banned_pattern '\.unwrap()' "$CORE_SRC" ".unwrap() in core (prefer ? or expect)" "warning"

# ── Summary ────────────────────────────────────────────────────────────
echo ""
echo "========================================"
if [ "$FAILURES" -gt 0 ]; then
    fail "Quality gate FAILED: $FAILURES check(s) failed, $WARNINGS warning(s)"
    echo "========================================"
    exit 1
else
    if [ "$WARNINGS" -gt 0 ]; then
        ok "Quality gate PASSED with $WARNINGS warning(s)"
    else
        ok "Quality gate PASSED (all checks clean)"
    fi
    echo "========================================"
    exit 0
fi
