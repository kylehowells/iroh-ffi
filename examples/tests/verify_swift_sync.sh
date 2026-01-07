#!/bin/bash
# Verify that Swift example files are in sync with IrohLib/Sources versions
#
# The files in IrohLib/Sources/*/main.swift are the authoritative versions
# used by Swift Package Manager. The copies in examples/ should match.

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$SCRIPT_DIR/common.sh"

# File pairs to check: (IrohLib source, examples copy)
# Note: Swift Package Manager can use any .swift file name, not just main.swift
declare -a FILE_PAIRS=(
    "IrohLib/Sources/GossipChat/GossipChat.swift:examples/GossipChat.swift"
    "IrohLib/Sources/BlobDemo/main.swift:examples/BlobDemo.swift"
    "IrohLib/Sources/DocDemo/main.swift:examples/DocDemo.swift"
)

verify_file_pair() {
    local pair="$1"
    local source_file="${pair%%:*}"
    local copy_file="${pair##*:}"

    local source_path="$PROJECT_ROOT/$source_file"
    local copy_path="$PROJECT_ROOT/$copy_file"

    local test_name="$source_file ↔ $copy_file"

    # Check if both files exist
    if [ ! -f "$source_path" ]; then
        log_fail "$test_name - Source file missing: $source_file"
        fail_test "$test_name"
        return 1
    fi

    if [ ! -f "$copy_path" ]; then
        log_fail "$test_name - Copy file missing: $copy_file"
        fail_test "$test_name"
        return 1
    fi

    # Compare files
    if diff -q "$source_path" "$copy_path" > /dev/null 2>&1; then
        pass_test "$test_name"
        return 0
    else
        log_fail "$test_name - Files differ!"
        echo ""
        echo "Differences:"
        diff "$source_path" "$copy_path" | head -20
        echo ""
        echo "To sync, run:"
        echo "  cp \"$source_path\" \"$copy_path\""
        fail_test "$test_name"
        return 1
    fi
}

main() {
    echo "=========================================="
    echo "Swift File Sync Verification"
    echo "=========================================="
    echo ""
    log_info "Verifying IrohLib/Sources and examples/ are in sync..."
    echo ""

    for pair in "${FILE_PAIRS[@]}"; do
        verify_file_pair "$pair" || true
    done

    echo ""
    print_summary
}

main "$@"
