#!/bin/bash
# PSM Player - Complete Build Script
# ===================================
#
# Builds all components of the PSM Player frequency analysis system:
# - Rust crates (core, frequency, cli)
# - WebAssembly module
# - Python bindings
# - Frontend components
#
# Usage:
#   ./scripts/build-all.sh          # Build everything
#   ./scripts/build-all.sh --rust   # Build only Rust
#   ./scripts/build-all.sh --wasm   # Build only WASM
#   ./scripts/build-all.sh --python # Build only Python
#   ./scripts/build-all.sh --test   # Build and test

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Script directory
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"

cd "$PROJECT_ROOT"

# Helper functions
log_info() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

log_success() {
    echo -e "${GREEN}[SUCCESS]${NC} $1"
}

log_warning() {
    echo -e "${YELLOW}[WARNING]${NC} $1"
}

log_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

# Build Rust crates
build_rust() {
    log_info "Building Rust crates..."
    cargo build --release --workspace
    log_success "Rust crates built successfully"
}

# Build WASM
build_wasm() {
    log_info "Building WebAssembly module..."

    # Check for wasm-pack
    if ! command -v wasm-pack &> /dev/null; then
        log_warning "wasm-pack not found, installing..."
        cargo install wasm-pack
    fi

    cd crates/psm-player-wasm
    wasm-pack build --target web --release
    cd "$PROJECT_ROOT"

    log_success "WASM module built at: crates/psm-player-wasm/pkg/"
}

# Build Python bindings
build_python() {
    log_info "Building Python bindings..."

    # Check for maturin
    if ! command -v maturin &> /dev/null; then
        log_warning "maturin not found, installing..."
        pip install maturin
    fi

    cd crates/psm-player-python
    maturin develop --release
    cd "$PROJECT_ROOT"

    log_success "Python module installed: psm_frequency"
}

# Run tests
run_tests() {
    log_info "Running tests..."

    # Rust tests
    log_info "Running Rust tests..."
    cargo test --workspace --all-features

    # Python tests (if available)
    if [ -d "crates/psm-player-python/tests" ]; then
        log_info "Running Python tests..."
        cd crates/psm-player-python
        python -m pytest tests/ -v || log_warning "Python tests failed or not found"
        cd "$PROJECT_ROOT"
    fi

    log_success "All tests completed"
}

# Build documentation
build_docs() {
    log_info "Building documentation..."
    cargo doc --workspace --all-features --no-deps
    log_success "Documentation built at: target/doc/"
}

# Install CLI
install_cli() {
    log_info "Installing CLI tool..."
    cargo install --path crates/psm-player-cli
    log_success "CLI installed: psm-player"
}

# Print usage
usage() {
    echo "PSM Player Build Script"
    echo ""
    echo "Usage: $0 [OPTIONS]"
    echo ""
    echo "Options:"
    echo "  --all       Build everything (default)"
    echo "  --rust      Build Rust crates only"
    echo "  --wasm      Build WASM module only"
    echo "  --python    Build Python bindings only"
    echo "  --test      Build and run tests"
    echo "  --docs      Build documentation"
    echo "  --install   Install CLI tool"
    echo "  --help      Show this help"
}

# Main
main() {
    local build_rust=false
    local build_wasm=false
    local build_python=false
    local run_test=false
    local build_documentation=false
    local do_install=false
    local build_all=true

    # Parse arguments
    while [[ $# -gt 0 ]]; do
        case $1 in
            --rust)
                build_rust=true
                build_all=false
                shift
                ;;
            --wasm)
                build_wasm=true
                build_all=false
                shift
                ;;
            --python)
                build_python=true
                build_all=false
                shift
                ;;
            --test)
                run_test=true
                shift
                ;;
            --docs)
                build_documentation=true
                shift
                ;;
            --install)
                do_install=true
                shift
                ;;
            --all)
                build_all=true
                shift
                ;;
            --help|-h)
                usage
                exit 0
                ;;
            *)
                log_error "Unknown option: $1"
                usage
                exit 1
                ;;
        esac
    done

    echo ""
    echo "=========================================="
    echo "  PSM Player - Build System"
    echo "=========================================="
    echo ""

    # Execute builds
    if $build_all || $build_rust; then
        build_rust
    fi

    if $build_all || $build_wasm; then
        build_wasm
    fi

    if $build_all || $build_python; then
        build_python
    fi

    if $run_test; then
        run_tests
    fi

    if $build_documentation; then
        build_docs
    fi

    if $do_install; then
        install_cli
    fi

    echo ""
    echo "=========================================="
    log_success "Build complete!"
    echo "=========================================="
    echo ""
    echo "Next steps:"
    echo "  - Run tests:      cargo test --workspace"
    echo "  - Run CLI:        cargo run -p psm-player-cli -- --help"
    echo "  - Use Python:     python -c 'import psm_frequency; print(psm_frequency.__version__)'"
    echo "  - Use WASM:       See crates/psm-player-wasm/examples/"
    echo ""
}

main "$@"
