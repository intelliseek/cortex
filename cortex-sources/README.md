# Cortex Sources

[![Crates.io](https://img.shields.io/crates/v/cortex-sources.svg)](https://crates.io/crates/cortex-sources)
[![Documentation](https://docs.rs/cortex-sources/badge.svg)](https://docs.rs/cortex-sources)
[![License: Apache](https://img.shields.io/badge/License-Apache-blue.svg)](https://opensource.org/license/apache-2-0)

Ready-made source implementations for various data inputs that can be used with the cortex-ai processing framework.

## Features

- File system sources
- More coming soon...

## Usage

Add this to your `Cargo.toml`:

```toml
[dependencies]
cortex-sources = "0.1.0"
```

### Example

```rust
use cortex_sources::fs::FileSource;
use cortex_ai::composer::flow::Source;

#[tokio::main]
async fn main() {
    let source = FileSource::new("path/to/file.txt");
    let content = source.read().await.unwrap();
    println!("File content: {}", content);
}
```

## License

This project is licensed under the Apache 2.0 License - see the [LICENSE](../LICENSE) file for details. 