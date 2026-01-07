#!/bin/bash
# Master test runner for iroh-ffi cross-language tests
#
# Runs all protocol tests (Gossip, Blobs, Docs) across all language combinations
#
# Usage:
#   ./run_all_tests.sh           # Run all tests
#   ./run_all_tests.sh gossip    # Run only gossip tests
#   ./run_all_tests.sh blobs     # Run only blob tests
#   ./run_all_tests.sh docs      # Run only document tests

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
CYAN='\033[0;36m'
NC='\033[0m'

# Track overall results
TOTAL_PASSED=0
TOTAL_FAILED=0
TOTAL_SKIPPED=0

print_banner() {
    echo ""
    echo -e "${CYAN}╔══════════════════════════════════════════════════════════════╗${NC}"
    echo -e "${CYAN}║                                                              ║${NC}"
    echo -e "${CYAN}║          ${NC}Iroh FFI Cross-Language Test Suite${CYAN}                 ║${NC}"
    echo -e "${CYAN}║                                                              ║${NC}"
    echo -e "${CYAN}║  ${NC}Testing: Rust ↔ Python ↔ Swift${CYAN}                            ║${NC}"
    echo -e "${CYAN}║  ${NC}Protocols: Gossip, Blobs, Documents${CYAN}                       ║${NC}"
    echo -e "${CYAN}║                                                              ║${NC}"
    echo -e "${CYAN}╚══════════════════════════════════════════════════════════════╝${NC}"
    echo ""
}

run_test_suite() {
    local name="$1"
    local script="$2"

    echo ""
    echo -e "${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
    echo -e "${BLUE}  Running: $name${NC}"
    echo -e "${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
    echo ""

    if bash "$script"; then
        echo -e "${GREEN}✓ $name completed${NC}"
    else
        echo -e "${RED}✗ $name had failures${NC}"
    fi
}

print_final_summary() {
    echo ""
    echo -e "${CYAN}╔══════════════════════════════════════════════════════════════╗${NC}"
    echo -e "${CYAN}║                    FINAL TEST SUMMARY                        ║${NC}"
    echo -e "${CYAN}╚══════════════════════════════════════════════════════════════╝${NC}"
    echo ""
    echo "  All test suites completed."
    echo ""
    echo "  Review individual test outputs above for detailed results."
    echo ""
    echo -e "${CYAN}══════════════════════════════════════════════════════════════════${NC}"
}

main() {
    print_banner

    local test_filter="${1:-all}"

    cd "$SCRIPT_DIR"

    # Make scripts executable
    chmod +x common.sh test_gossip.sh test_blobs.sh test_docs.sh verify_swift_sync.sh 2>/dev/null || true

    case "$test_filter" in
        gossip)
            run_test_suite "Gossip Messaging Tests" "$SCRIPT_DIR/test_gossip.sh"
            ;;
        blobs|blob)
            run_test_suite "Blob Transfer Tests" "$SCRIPT_DIR/test_blobs.sh"
            ;;
        docs|doc|documents)
            run_test_suite "Document Sync Tests" "$SCRIPT_DIR/test_docs.sh"
            ;;
        sync|verify)
            run_test_suite "Swift File Sync Verification" "$SCRIPT_DIR/verify_swift_sync.sh"
            ;;
        all|"")
            run_test_suite "Swift File Sync Verification" "$SCRIPT_DIR/verify_swift_sync.sh"
            run_test_suite "Gossip Messaging Tests" "$SCRIPT_DIR/test_gossip.sh"
            run_test_suite "Blob Transfer Tests" "$SCRIPT_DIR/test_blobs.sh"
            run_test_suite "Document Sync Tests" "$SCRIPT_DIR/test_docs.sh"
            ;;
        *)
            echo "Usage: $0 [gossip|blobs|docs|sync|all]"
            echo ""
            echo "Options:"
            echo "  gossip  - Run gossip messaging tests only"
            echo "  blobs   - Run blob transfer tests only"
            echo "  docs    - Run document sync tests only"
            echo "  sync    - Verify Swift files are in sync"
            echo "  all     - Run all tests (default)"
            exit 1
            ;;
    esac

    print_final_summary
}

main "$@"
