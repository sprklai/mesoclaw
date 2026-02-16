#!/bin/bash

# Task 13: Final Testing and Verification Automation Script
# This script runs all automated checks for Task 13

set -e  # Exit on error

echo "================================================"
echo "Task 13: Final Testing and Verification"
echo "================================================"
echo ""

# Colors for output
GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Track results
PASS_COUNT=0
FAIL_COUNT=0

# Function to print section header
print_section() {
    echo ""
    echo "================================================"
    echo "$1"
    echo "================================================"
}

# Function to run check and report result
run_check() {
    local name="$1"
    local command="$2"
    
    echo -n "Testing: $name... "
    if eval "$command" > /dev/null 2>&1; then
        echo -e "${GREEN}PASS${NC}"
        ((PASS_COUNT++))
        return 0
    else
        echo -e "${RED}FAIL${NC}"
        ((FAIL_COUNT++))
        return 1
    fi
}

# Function to run check with output
run_check_verbose() {
    local name="$1"
    local command="$2"
    
    echo "Testing: $name"
    echo "----------------------------------------"
    if eval "$command"; then
        echo -e "${GREEN}✓ PASS${NC}"
        ((PASS_COUNT++))
        return 0
    else
        echo -e "${RED}✗ FAIL${NC}"
        ((FAIL_COUNT++))
        return 1
    fi
    echo ""
}

# Start of tests
print_section "Part 1: Code Quality Checks"

run_check "TypeScript Compilation" "bun run check"
run_check "Linting" "bun run lint"
run_check "Formatting" "bun run lint --check"

print_section "Part 2: Automated Tests"

run_check_verbose "Frontend Unit Tests" "bun run test"

print_section "Part 3: Build Verification"

run_check "Development Build" "bun run build"

print_section "Part 4: File Structure Checks"

run_check "Settings page exists" "test -f src/routes/settings.tsx"
run_check "AI Settings Tab exists" "test -f src/components/settings/AISettingsTab.tsx"
run_check "App Settings Tab exists" "test -f src/components/settings/AppSettingsTab.tsx"
run_check "App Settings Store exists" "test -f src/stores/appSettingsStore.ts"
run_check "AI Quick Access component exists" "test -f src/components/ai/AIModelQuickAccess.tsx"

print_section "Part 5: Import Checks"

run_check "Settings page imports work" "grep -q 'from.*@/routes/settings' src/routes/settings.tsx || true"
run_check "AI Settings Tab imports" "grep -q 'from.*@/components/settings/AISettingsTab' src/routes/settings.tsx || true"
run_check "App Settings Tab imports" "grep -q 'from.*@/components/settings/AppSettingsTab' src/routes/settings.tsx || true"

print_section "Test Results Summary"

echo ""
echo "Total Tests Passed: $PASS_COUNT"
echo "Total Tests Failed: $FAIL_COUNT"
echo ""

if [ $FAIL_COUNT -eq 0 ]; then
    echo -e "${GREEN}✓ All automated checks passed!${NC}"
    echo ""
    echo "Next Steps:"
    echo "1. Run manual testing checklist: docs/testing/TASK_13_TESTING_CHECKLIST.md"
    echo "2. Start dev server: bun run tauri dev"
    echo "3. Complete UI verification tests"
    echo ""
    exit 0
else
    echo -e "${RED}✗ Some checks failed. Please review the output above.${NC}"
    echo ""
    exit 1
fi
