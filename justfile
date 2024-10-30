# List all available commands
default:
    @just --list

# Install all required tools
install-tools:
    cargo install cargo-nextest
    cargo install cargo-llvm-cov
    rustup component add llvm-tools-preview

# Build the project
build:
    cargo build

# Run tests with nextest
test:
    cargo nextest run

test-ci:
    cargo nextest run --profile ci  

test-doc:
    cargo test --doc --package cortex-ai -- --show-output 

# Run tests with coverage
coverage:
    cargo llvm-cov nextest --lcov --output-path target/llvm-cov-target/lcov.info --package cortex-ai
    @echo "Coverage report generated in target/llvm-cov-target/lcov.info"

# Run benchmarks
bench:
    cargo bench

# Clean up all build artifacts
clean:
    cargo clean

# Format code
fmt:
    cargo fmt -- --check
    cargo fmt --all -- --check

# Run clippy
lint:
    cargo clippy -- -D warnings -W clippy::all -W clippy::pedantic -W clippy::nursery
    cargo clippy --all-targets -- -D warnings -W clippy::all
    cargo clippy --all-features -- -D warnings -W clippy::all 


# Run all checks (format, lint, test)
check: fmt lint test 

# Run the sample project
sample:
    cd cortex-sample && cargo run