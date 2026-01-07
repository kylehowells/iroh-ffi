#!/bin/bash
# Blob transfer cross-language tests
#
# Tests all 9 combinations of Rust/Python/Swift blob transfer

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$SCRIPT_DIR/common.sh"

# Test message
TEST_DATA="Hello from iroh-ffi blob test! $(date +%s)"

# Test blob transfer between two languages
test_blob_transfer() {
    local sender_lang="$1"
    local receiver_lang="$2"

    local test_name="$(lang_name $sender_lang) → $(lang_name $receiver_lang)"
    log_test "Blob: $test_name"

    local test_dir
    test_dir=$(create_test_dir "blob-${sender_lang}-${receiver_lang}")
    local sender_log="$test_dir/sender.log"
    local receiver_log="$test_dir/receiver.log"
    local sender_pid=""

    # Cleanup function
    cleanup() {
        cleanup_process "$sender_pid"
        cleanup_test_dir "$test_dir"
    }
    trap cleanup EXIT

    # Start sender
    case "$sender_lang" in
        rust)
            cd "$PROJECT_ROOT"
            timeout 60 cargo run --example blob_demo -- send-bytes "$TEST_DATA" > "$sender_log" 2>&1 &
            sender_pid=$!
            ;;
        python)
            cd "$PROJECT_ROOT"
            PYTHONUNBUFFERED=1 timeout 60 $PYTHON_CMD examples/blob_demo.py send-bytes "$TEST_DATA" > "$sender_log" 2>&1 &
            sender_pid=$!
            ;;
        swift)
            cd "$PROJECT_ROOT/IrohLib"
            timeout 60 swift run BlobDemo send-bytes "$TEST_DATA" > "$sender_log" 2>&1 &
            sender_pid=$!
            ;;
    esac

    # Wait for ticket (blob tickets start with "blob" followed by alphanumeric)
    if ! wait_for_pattern "$sender_log" "BLOB TICKET" 45; then
        log_fail "$test_name - Sender failed to produce ticket"
        cat "$sender_log" 2>/dev/null || true
        fail_test "$test_name"
        trap - EXIT
        cleanup
        return 1
    fi

    local ticket
    ticket=$(grep -oE 'blob[a-z0-9]{100,}' "$sender_log" 2>/dev/null | head -1)
    if [ -z "$ticket" ]; then
        log_fail "$test_name - Could not extract ticket"
        fail_test "$test_name"
        trap - EXIT
        cleanup
        return 1
    fi

    # Start receiver (provide a destination file path)
    local dest_file="$test_dir/received.bin"
    case "$receiver_lang" in
        rust)
            cd "$PROJECT_ROOT"
            timeout 30 cargo run --example blob_demo -- receive "$ticket" "$dest_file" > "$receiver_log" 2>&1 || true
            ;;
        python)
            cd "$PROJECT_ROOT"
            PYTHONUNBUFFERED=1 timeout 30 $PYTHON_CMD examples/blob_demo.py receive "$ticket" "$dest_file" > "$receiver_log" 2>&1 || true
            ;;
        swift)
            cd "$PROJECT_ROOT/IrohLib"
            timeout 30 swift run BlobDemo receive "$ticket" "$dest_file" > "$receiver_log" 2>&1 || true
            ;;
    esac

    # Verify received content - check both the log and the received file
    local found=0
    if grep -q "$TEST_DATA" "$receiver_log" 2>/dev/null; then
        found=1
    elif [ -f "$dest_file" ] && grep -q "$TEST_DATA" "$dest_file" 2>/dev/null; then
        found=1
    fi

    if [ $found -eq 1 ]; then
        pass_test "$test_name"
        trap - EXIT
        cleanup
        return 0
    else
        log_fail "$test_name - Content not received correctly"
        echo "Expected: $TEST_DATA"
        echo "Receiver output:"
        cat "$receiver_log" 2>/dev/null || echo "(no output)"
        echo "Received file:"
        cat "$dest_file" 2>/dev/null || echo "(no file)"
        fail_test "$test_name"
        trap - EXIT
        cleanup
        return 1
    fi
}

# Main
main() {
    echo "=========================================="
    echo "Blob Transfer Cross-Language Tests"
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
    log_info "Running blob transfer tests..."
    echo ""

    # All language combinations
    local languages=("rust" "python" "swift")

    for sender in "${languages[@]}"; do
        for receiver in "${languages[@]}"; do
            # Skip if Python not available
            if [ -z "$PYTHON_CMD" ] && { [ "$sender" = "python" ] || [ "$receiver" = "python" ]; }; then
                skip_test "$(lang_name $sender) → $(lang_name $receiver) (Python not available)"
                continue
            fi

            test_blob_transfer "$sender" "$receiver" || true
            echo ""
        done
    done

    print_summary
}

main "$@"
