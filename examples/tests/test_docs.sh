#!/bin/bash
# Document sync cross-language tests
#
# Tests all 9 combinations of Rust/Python/Swift document sync

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$SCRIPT_DIR/common.sh"

# Test document sync between two languages
test_doc_sync() {
    local creator_lang="$1"
    local joiner_lang="$2"

    local test_name="$(lang_name $creator_lang) → $(lang_name $joiner_lang)"
    log_test "Docs: $test_name"

    local test_dir
    test_dir=$(create_test_dir "doc-${creator_lang}-${joiner_lang}")
    local creator_log="$test_dir/creator.log"
    local joiner_log="$test_dir/joiner.log"
    local creator_pid=""

    # Cleanup function
    cleanup() {
        cleanup_process "$creator_pid"
        cleanup_test_dir "$test_dir"
    }
    trap cleanup EXIT

    # Determine expected greeting based on creator language
    local expected_greeting
    case "$creator_lang" in
        rust) expected_greeting="Hello from Rust!" ;;
        python) expected_greeting="Hello from Python!" ;;
        swift) expected_greeting="Hello from Swift!" ;;
    esac

    # Start creator (keeps running to allow sync)
    case "$creator_lang" in
        rust)
            cd "$PROJECT_ROOT"
            (sleep 45 | timeout 50 cargo run --example doc_demo -- create) > "$creator_log" 2>&1 &
            creator_pid=$!
            ;;
        python)
            cd "$PROJECT_ROOT"
            (sleep 45 | PYTHONUNBUFFERED=1 timeout 50 $PYTHON_CMD examples/doc_demo.py create) > "$creator_log" 2>&1 &
            creator_pid=$!
            ;;
        swift)
            cd "$PROJECT_ROOT/IrohLib"
            (sleep 45 | timeout 50 swift run DocDemo create) > "$creator_log" 2>&1 &
            creator_pid=$!
            ;;
    esac

    # Wait for ticket (doc tickets start with "docaaac")
    if ! wait_for_pattern "$creator_log" "DOC TICKET" 45; then
        log_fail "$test_name - Creator failed to produce ticket"
        cat "$creator_log" 2>/dev/null || true
        fail_test "$test_name"
        trap - EXIT
        cleanup
        return 1
    fi

    local ticket
    ticket=$(grep -oE 'docaaac[a-z0-9]{100,}' "$creator_log" 2>/dev/null | head -1)
    if [ -z "$ticket" ]; then
        log_fail "$test_name - Could not extract ticket"
        fail_test "$test_name"
        trap - EXIT
        cleanup
        return 1
    fi

    # Start joiner
    case "$joiner_lang" in
        rust)
            cd "$PROJECT_ROOT"
            (sleep 15 | timeout 25 cargo run --example doc_demo -- join "$ticket") > "$joiner_log" 2>&1 || true
            ;;
        python)
            cd "$PROJECT_ROOT"
            (sleep 15 | PYTHONUNBUFFERED=1 timeout 25 $PYTHON_CMD examples/doc_demo.py join "$ticket") > "$joiner_log" 2>&1 || true
            ;;
        swift)
            cd "$PROJECT_ROOT/IrohLib"
            (sleep 15 | timeout 25 swift run DocDemo join "$ticket") > "$joiner_log" 2>&1 || true
            ;;
    esac

    # Verify joiner received the greeting
    if grep -q "$expected_greeting" "$joiner_log" 2>/dev/null; then
        pass_test "$test_name"
        trap - EXIT
        cleanup
        return 0
    else
        log_fail "$test_name - Document not synced correctly"
        echo "Expected to find: $expected_greeting"
        echo "Joiner output:"
        cat "$joiner_log" 2>/dev/null || echo "(no output)"
        fail_test "$test_name"
        trap - EXIT
        cleanup
        return 1
    fi
}

# Main
main() {
    echo "=========================================="
    echo "Document Sync Cross-Language Tests"
    echo "=========================================="
    echo ""

    # Check prerequisites
    log_info "Checking prerequisites..."

    PYTHON_CMD=$(check_python) || {
        log_skip "Python not available"
        PYTHON_CMD=""
    }

    # Build if needed
    build_rust
    build_swift

    echo ""
    log_info "Running document sync tests..."
    echo ""

    # All language combinations
    local languages=("rust" "python" "swift")

    for creator in "${languages[@]}"; do
        for joiner in "${languages[@]}"; do
            # Skip if Python not available
            if [ -z "$PYTHON_CMD" ] && { [ "$creator" = "python" ] || [ "$joiner" = "python" ]; }; then
                skip_test "$(lang_name $creator) → $(lang_name $joiner) (Python not available)"
                continue
            fi

            test_doc_sync "$creator" "$joiner" || true
            echo ""
        done
    done

    print_summary
}

main "$@"
