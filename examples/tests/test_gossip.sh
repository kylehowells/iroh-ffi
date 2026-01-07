#!/bin/bash
# Gossip messaging cross-language tests
#
# Tests all 9 combinations of Rust/Python/Swift gossip communication
#
# Gossip demos use topic + node address sharing:
#   Sender: creates topic, outputs TOPIC, NODE_ID, RELAY_URL
#   Receiver: joins with TOPIC NODE_ID RELAY_URL

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$SCRIPT_DIR/common.sh"

# Test gossip messaging between two languages
test_gossip() {
    local sender_lang="$1"
    local receiver_lang="$2"

    local test_name="$(lang_name $sender_lang) → $(lang_name $receiver_lang)"
    log_test "Gossip: $test_name"

    local test_dir
    test_dir=$(create_test_dir "gossip-${sender_lang}-${receiver_lang}")
    local sender_log="$test_dir/sender.log"
    local receiver_log="$test_dir/receiver.log"
    local sender_in="$test_dir/sender_in"
    local sender_pid=""
    local receiver_pid=""

    # Unique test message
    local test_msg="GossipTest_${sender_lang}_${receiver_lang}_$$"

    # Cleanup function
    cleanup() {
        exec 3>&- 2>/dev/null || true
        cleanup_process "$sender_pid"
        cleanup_process "$receiver_pid"
        rm -f "$sender_in"
        cleanup_test_dir "$test_dir"
    }
    trap cleanup EXIT

    # Create FIFO for sender stdin control
    mkfifo "$sender_in"

    # Start sender (creates topic, reads from FIFO)
    case "$sender_lang" in
        rust)
            cd "$PROJECT_ROOT"
            timeout 60 cargo run --example gossip_chat < "$sender_in" > "$sender_log" 2>&1 &
            sender_pid=$!
            ;;
        python)
            cd "$PROJECT_ROOT"
            PYTHONUNBUFFERED=1 timeout 60 $PYTHON_CMD examples/gossip_chat.py < "$sender_in" > "$sender_log" 2>&1 &
            sender_pid=$!
            ;;
        swift)
            cd "$PROJECT_ROOT/IrohLib"
            timeout 60 swift run GossipChat < "$sender_in" > "$sender_log" 2>&1 &
            sender_pid=$!
            ;;
    esac

    # Open FIFO for writing
    exec 3>"$sender_in"

    # Wait for sender to start and print topic/node info
    if ! wait_for_pattern "$sender_log" "Share your node ID" 30; then
        log_fail "$test_name - Sender failed to start"
        cat "$sender_log" 2>/dev/null || true
        fail_test "$test_name"
        trap - EXIT
        cleanup
        return 1
    fi

    # Extract topic, node ID, and relay URL from sender output
    local topic node_id relay_url
    topic=$(grep "^Topic:" "$sender_log" 2>/dev/null | head -1 | awk '{print $2}')
    node_id=$(grep "Share your node ID:" "$sender_log" 2>/dev/null | head -1 | awk '{print $5}')
    relay_url=$(grep "Share your relay URL:" "$sender_log" 2>/dev/null | head -1 | awk '{print $5}')

    if [ -z "$topic" ] || [ -z "$node_id" ]; then
        log_fail "$test_name - Could not extract topic/node info"
        cat "$sender_log" 2>/dev/null || true
        fail_test "$test_name"
        trap - EXIT
        cleanup
        return 1
    fi

    # Start receiver with topic, node_id, relay_url
    case "$receiver_lang" in
        rust)
            cd "$PROJECT_ROOT"
            (sleep 25; echo "/quit") | \
                timeout 35 cargo run --example gossip_chat -- "$topic" "$node_id" "$relay_url" > "$receiver_log" 2>&1 &
            receiver_pid=$!
            ;;
        python)
            cd "$PROJECT_ROOT"
            (sleep 25; echo "/quit") | \
                PYTHONUNBUFFERED=1 timeout 35 $PYTHON_CMD examples/gossip_chat.py "$topic" "$node_id" "$relay_url" > "$receiver_log" 2>&1 &
            receiver_pid=$!
            ;;
        swift)
            cd "$PROJECT_ROOT/IrohLib"
            (sleep 25; echo "/quit") | \
                timeout 35 swift run GossipChat "$topic" "$node_id" "$relay_url" > "$receiver_log" 2>&1 &
            receiver_pid=$!
            ;;
    esac

    # Wait for peers to connect
    log_info "Waiting for peers to connect..."
    local connected=0
    for i in {1..20}; do
        if grep -q "Peer connected\|NeighborUp\|peer_connected" "$sender_log" 2>/dev/null; then
            connected=1
            break
        fi
        sleep 1
    done

    if [ $connected -eq 0 ]; then
        log_fail "$test_name - Peers did not connect"
        echo "Sender log:"
        tail -20 "$sender_log"
        echo "Receiver log:"
        tail -20 "$receiver_log"
        fail_test "$test_name"
        trap - EXIT
        cleanup
        return 1
    fi

    # Send the test message
    log_info "Sending test message..."
    echo "$test_msg" >&3

    # Wait for message delivery
    sleep 5

    # Close sender
    echo "/quit" >&3
    exec 3>&-

    # Wait for processes
    wait $sender_pid 2>/dev/null || true
    wait $receiver_pid 2>/dev/null || true

    # Check if receiver got the message
    if grep -q "$test_msg" "$receiver_log" 2>/dev/null; then
        pass_test "$test_name"
        trap - EXIT
        cleanup
        return 0
    else
        log_fail "$test_name - Message not received"
        echo "Expected message: $test_msg"
        echo ""
        echo "Receiver output (last 30 lines):"
        tail -30 "$receiver_log" 2>/dev/null || echo "(no output)"
        fail_test "$test_name"
        trap - EXIT
        cleanup
        return 1
    fi
}

# Main
main() {
    echo "=========================================="
    echo "Gossip Messaging Cross-Language Tests"
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
    log_info "Running gossip messaging tests..."
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

            test_gossip "$sender" "$receiver" || true
            echo ""
        done
    done

    print_summary
}

main "$@"
