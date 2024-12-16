# Cortex Sinks

Ready-made sink implementations for the cortex-ai processing framework.

## Features

- File system sinks for writing data to files
- Database sinks for storing data in various databases
- Message queue sinks for publishing to message brokers
- API client sinks for sending data to external services
- Metrics sinks for monitoring and observability
- Vector store sinks for storing embeddings and vectors

## Usage

Add this to your `Cargo.toml`:

```toml
[dependencies]
cortex-sinks = "0.1"
```

## Example

```rust
use cortex_ai::Flow;
use cortex_sinks::file::FileSink;
use cortex_sinks::types::FileSinkConfig;
use std::path::PathBuf;

let config = FileSinkConfig {
    path: PathBuf::from("output.txt"),
    append: true,
};

let flow = Flow::new()
    // Add your source and processors here
    .sink(FileSink::new(config));
```

## License

This project is licensed under the Apache 2.0 License - see the [LICENSE](../LICENSE) file for details. 