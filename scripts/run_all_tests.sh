#!/bin/bash

# Comprehensive test runner for Straico Proxy
# This script runs all tests and generates a summary report

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

echo -e "${BLUE}=== Straico Proxy Comprehensive Test Suite ===${NC}"
echo ""

# Change to project root
cd "$(dirname "$0")/.."

# Function to run a test category
run_test_category() {
    local category="$1"
    local command="$2"
    
    echo -e "${YELLOW}Running $category...${NC}"
    if eval "$command"; then
        echo -e "${GREEN}✓ $category PASSED${NC}"
        return 0
    else
        echo -e "${RED}✗ $category FAILED${NC}"
        return 1
    fi
}

# Initialize counters
total_tests=0
passed_tests=0

# 1. Unit Tests
echo -e "${BLUE}1. Unit Tests${NC}"
total_tests=$((total_tests + 1))
if run_test_category "Client Unit Tests" "cd client && cargo test --lib"; then
    passed_tests=$((passed_tests + 1))
fi

total_tests=$((total_tests + 1))
if run_test_category "Proxy Unit Tests" "cd proxy && cargo test --lib"; then
    passed_tests=$((passed_tests + 1))
fi

echo ""

# 2. Integration Tests
echo -e "${BLUE}2. Integration Tests${NC}"
total_tests=$((total_tests + 1))
if run_test_category "End-to-End Tests" "cd proxy && cargo test --test end_to_end_tests"; then
    passed_tests=$((passed_tests + 1))
fi

total_tests=$((total_tests + 1))
if run_test_category "OpenAI Compatibility Tests" "cd proxy && cargo test --test openai_compatibility_tests"; then
    passed_tests=$((passed_tests + 1))
fi

total_tests=$((total_tests + 1))
if run_test_category "Performance Tests" "cd proxy && cargo test --test performance_tests"; then
    passed_tests=$((passed_tests + 1))
fi

echo ""

# 3. Configuration Tests
echo -e "${BLUE}3. Configuration Tests${NC}"
total_tests=$((total_tests + 1))
if run_test_category "Configuration Management" "cd proxy && cargo test config_manager"; then
    passed_tests=$((passed_tests + 1))
fi

total_tests=$((total_tests + 1))
if run_test_category "Feature Flags" "cd proxy && cargo test feature_flag"; then
    passed_tests=$((passed_tests + 1))
fi

echo ""

# 4. Content Conversion Tests
echo -e "${BLUE}4. Content Conversion Tests${NC}"
total_tests=$((total_tests + 1))
if run_test_category "Content Conversion" "cd proxy && cargo test content_conversion"; then
    passed_tests=$((passed_tests + 1))
fi

total_tests=$((total_tests + 1))
if run_test_category "OpenAI Types" "cd proxy && cargo test openai_types"; then
    passed_tests=$((passed_tests + 1))
fi

echo ""

# 5. Compilation Tests
echo -e "${BLUE}5. Compilation Tests${NC}"
total_tests=$((total_tests + 1))
if run_test_category "Client Compilation" "cd client && cargo check"; then
    passed_tests=$((passed_tests + 1))
fi

total_tests=$((total_tests + 1))
if run_test_category "Proxy Compilation" "cd proxy && cargo check"; then
    passed_tests=$((passed_tests + 1))
fi

echo ""

# 6. Documentation Tests
echo -e "${BLUE}6. Documentation Tests${NC}"
total_tests=$((total_tests + 1))
if run_test_category "Client Documentation" "cd client && cargo doc --no-deps"; then
    passed_tests=$((passed_tests + 1))
fi

total_tests=$((total_tests + 1))
if run_test_category "Proxy Documentation" "cd proxy && cargo doc --no-deps"; then
    passed_tests=$((passed_tests + 1))
fi

echo ""

# 7. Linting and Formatting
echo -e "${BLUE}7. Code Quality${NC}"
total_tests=$((total_tests + 1))
if run_test_category "Client Formatting Check" "cd client && cargo fmt -- --check"; then
    passed_tests=$((passed_tests + 1))
fi

total_tests=$((total_tests + 1))
if run_test_category "Proxy Formatting Check" "cd proxy && cargo fmt -- --check"; then
    passed_tests=$((passed_tests + 1))
fi

# Note: Clippy might have warnings, so we'll make it non-failing for now
echo -e "${YELLOW}Running Clippy (warnings only)...${NC}"
cd client && cargo clippy -- -W clippy::all || true
cd ../proxy && cargo clippy -- -W clippy::all || true

echo ""

# 8. Test Coverage (if available)
echo -e "${BLUE}8. Test Coverage${NC}"
if command -v cargo-tarpaulin &> /dev/null; then
    total_tests=$((total_tests + 1))
    if run_test_category "Test Coverage" "cargo tarpaulin --out Stdout --timeout 120"; then
        passed_tests=$((passed_tests + 1))
    fi
else
    echo -e "${YELLOW}cargo-tarpaulin not installed, skipping coverage${NC}"
fi

echo ""

# Generate summary report
echo -e "${BLUE}=== Test Summary Report ===${NC}"
echo "Total test categories: $total_tests"
echo "Passed: $passed_tests"
echo "Failed: $((total_tests - passed_tests))"

if [ $passed_tests -eq $total_tests ]; then
    echo -e "${GREEN}🎉 All tests passed!${NC}"
    exit 0
else
    echo -e "${RED}❌ Some tests failed.${NC}"
    exit 1
fi