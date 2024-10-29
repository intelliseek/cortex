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

# Run tests with coverage
coverage:
    cargo llvm-cov nextest --lcov --output-path target/llvm-cov-target/lcov.info --package cortex-ai
    @echo "Coverage report generated in target/llvm-cov-target/lcov.info"


# Clean up all build artifacts
clean:
    cargo clean

# Format code
fmt:
    cargo fmt

# Run clippy
lint:
    cargo clippy -- -D warnings

# Run all checks (format, lint, test)
check: fmt lint test 

# Run the sample project
sample:
    cd cortex-sample && cargo run