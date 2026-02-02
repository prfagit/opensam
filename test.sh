#!/bin/bash
# OpenSAM Test Runner Script

set -e

echo "========================================"
echo "OpenSAM Test Suite Runner"
echo "========================================"
echo ""

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Function to run tests for a crate
run_crate_tests() {
    local crate=$1
    echo -e "${YELLOW}Testing $crate...${NC}"
    if cargo test -p $crate --quiet 2>/dev/null; then
        echo -e "${GREEN}✓ $crate passed${NC}"
        return 0
    else
        echo -e "${RED}✗ $crate failed${NC}"
        return 1
    fi
}

# Check if cargo is available
if ! command -v cargo &> /dev/null; then
    echo -e "${RED}Error: cargo is not installed${NC}"
    echo "Please install Rust: https://rustup.rs/"
    exit 1
fi

# Parse arguments
MODE=${1:-"all"}

# Show help
if [ "$MODE" == "help" ] || [ "$MODE" == "--help" ] || [ "$MODE" == "-h" ]; then
    echo "Usage: ./test.sh [MODE]"
    echo ""
    echo "Modes:"
    echo "  all       - Run all tests (default)"
    echo "  unit      - Run unit tests only"
    echo "  integration - Run integration tests only"
    echo "  config    - Run opensam-config tests"
    echo "  bus       - Run opensam-bus tests"
    echo "  provider  - Run opensam-provider tests"
    echo "  agent     - Run opensam-agent tests"
    echo "  session   - Run opensam-session tests"
    echo "  channels  - Run opensam-channels tests"
    echo "  cron      - Run opensam-cron tests"
    echo "  heartbeat - Run opensam-heartbeat tests"
    echo "  cli       - Run opensam CLI tests"
    echo "  help      - Show this help"
    echo ""
    echo "Examples:"
    echo "  ./test.sh              # Run all tests"
    echo "  ./test.sh agent        # Run agent tests only"
    echo "  ./test.sh integration  # Run integration tests only"
    exit 0
fi

# Run based on mode
case $MODE in
    all)
        echo "Running full test suite..."
        echo ""
        cargo test --workspace
        echo ""
        echo -e "${GREEN}========================================"
        echo "All tests passed!"
        echo "========================================${NC}"
        ;;
    
    unit)
        echo "Running unit tests..."
        cargo test --workspace --lib
        ;;
    
    integration)
        echo "Running integration tests..."
        cargo test --workspace --test '*'
        ;;
    
    config)
        run_crate_tests "opensam-config"
        ;;
    
    bus)
        run_crate_tests "opensam-bus"
        ;;
    
    provider)
        run_crate_tests "opensam-provider"
        ;;
    
    agent)
        run_crate_tests "opensam-agent"
        ;;
    
    session)
        run_crate_tests "opensam-session"
        ;;
    
    channels)
        run_crate_tests "opensam-channels"
        ;;
    
    cron)
        run_crate_tests "opensam-cron"
        ;;
    
    heartbeat)
        run_crate_tests "opensam-heartbeat"
        ;;
    
    cli)
        run_crate_tests "opensam"
        ;;
    
    *)
        echo -e "${RED}Unknown mode: $MODE${NC}"
        echo "Run './test.sh help' for usage information"
        exit 1
        ;;
esac

echo ""
echo "Done!"
