#!/bin/bash
# Common test utilities for iroh-ffi cross-language tests

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Project root directory
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"

# Test results
TESTS_PASSED=0
TESTS_FAILED=0
TESTS_SKIPPED=0

# Logging functions
log_info() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

log_success() {
    echo -e "${GREEN}[PASS]${NC} $1"
}

log_fail() {
    echo -e "${RED}[FAIL]${NC} $1"
}

log_skip() {
    echo -e "${YELLOW}[SKIP]${NC} $1"
}

log_test() {
    echo -e "${YELLOW}[TEST]${NC} $1"
}

# Record test result
pass_test() {
    log_success "$1"
    ((TESTS_PASSED++))
}

fail_test() {
    log_fail "$1"
    ((TESTS_FAILED++))
}

skip_test() {
    log_skip "$1"
    ((TESTS_SKIPPED++))
}

# Print test summary
print_summary() {
    echo ""
    echo "=========================================="
    echo "Test Summary"
    echo "=========================================="
    echo -e "${GREEN}Passed:${NC}  $TESTS_PASSED"
    echo -e "${RED}Failed:${NC}  $TESTS_FAILED"
    echo -e "${YELLOW}Skipped:${NC} $TESTS_SKIPPED"
    echo "=========================================="

    if [ $TESTS_FAILED -gt 0 ]; then
        return 1
    fi
    return 0
}

# Check if a command exists
check_command() {
    if ! command -v "$1" &> /dev/null; then
        return 1
    fi
    return 0
}

# Wait for a pattern in a file with timeout
wait_for_pattern() {
    local file="$1"
    local pattern="$2"
    local timeout="${3:-30}"
    local interval="${4:-0.5}"

    local elapsed=0
    while [ $elapsed -lt $timeout ]; do
        if [ -f "$file" ] && grep -q "$pattern" "$file" 2>/dev/null; then
            return 0
        fi
        sleep $interval
        elapsed=$((elapsed + 1))
    done
    return 1
}

# Extract ticket from output file
extract_ticket() {
    local file="$1"
    local prefix="$2"  # e.g., "docaaac", "blobaaac", or gossip ticket pattern

    grep -o "${prefix}[a-z0-9]*" "$file" 2>/dev/null | head -1
}

# Kill process and children
cleanup_process() {
    local pid="$1"
    if [ -n "$pid" ] && kill -0 "$pid" 2>/dev/null; then
        kill "$pid" 2>/dev/null || true
        wait "$pid" 2>/dev/null || true
    fi
}

# Create temp directory for test
create_test_dir() {
    local name="$1"
    local dir="/tmp/iroh-test-${name}-$$"
    mkdir -p "$dir"
    echo "$dir"
}

# Cleanup temp directory
cleanup_test_dir() {
    local dir="$1"
    if [ -d "$dir" ]; then
        rm -rf "$dir"
    fi
}

# Build Rust examples if needed
build_rust() {
    log_info "Building Rust examples..."
    cd "$PROJECT_ROOT"
    cargo build --examples 2>&1 | tail -3
}

# Build Swift if needed
build_swift() {
    log_info "Building Swift demos..."
    cd "$PROJECT_ROOT/IrohLib"
    swift build 2>&1 | tail -3
}

# Check Python environment
check_python() {
    if [ -f "$PROJECT_ROOT/.venv/bin/python3" ]; then
        echo "$PROJECT_ROOT/.venv/bin/python3"
        return 0
    elif check_command python3; then
        # Check if iroh module is available
        if python3 -c "import iroh" 2>/dev/null; then
            echo "python3"
            return 0
        fi
    fi
    return 1
}

# Run Rust demo
run_rust() {
    local example="$1"
    shift
    cd "$PROJECT_ROOT"
    cargo run --example "$example" -- "$@"
}

# Run Python demo
run_python() {
    local script="$1"
    shift
    local python_cmd
    python_cmd=$(check_python) || { echo "Python not available"; return 1; }
    cd "$PROJECT_ROOT"
    "$python_cmd" "examples/$script" "$@"
}

# Run Swift demo
run_swift() {
    local target="$1"
    shift
    cd "$PROJECT_ROOT/IrohLib"
    swift run "$target" "$@"
}

# Language display names
lang_name() {
    case "$1" in
        rust) echo "Rust" ;;
        python) echo "Python" ;;
        swift) echo "Swift" ;;
        *) echo "$1" ;;
    esac
}
