 # Cortex Project

A collection of high-performance, asynchronous data processing libraries built in Rust, designed for RAG (Retrieval-Augmented Generation) applications and AI workflows.

## Project Structure

### [cortex-ai](./cortex-ai/README.md)
[![Crates.io](https://img.shields.io/crates/v/cortex-ai.svg)](https://crates.io/crates/cortex-ai)
[![Documentation](https://docs.rs/cortex-ai/badge.svg)](https://docs.rs/cortex-ai)
[![License: Apache](https://img.shields.io/badge/License-Apache-blue.svg)](https://opensource.org/license/apache-2-0)
[![Coverage Status](https://codecov.io/gh/intelliseek/cortex-ai/branch/main/graph/badge.svg)](https://codecov.io/gh/intelliseek/cortex-ai)

The core library providing the flow-based processing framework. Features include:
- Async data processing pipelines
- Conditional branching
- Error handling and feedback mechanisms
- Type-safe data processing
- Comprehensive testing and documentation

### cortex-processors (Coming Soon)
A collection of ready-to-use processors for common data processing tasks:
- Text preprocessing
- Vector operations
- Data transformation
- Filtering and validation
- Integration with popular AI models

### cortex-sources (Coming Soon)
Ready-made source implementations for various data inputs:
- File system sources
- Database connectors
- Message queue integrations
- API endpoints
- Streaming data sources

### cortex-conditions (Coming Soon)
A library of reusable conditions for flow control:
- Data validation conditions
- Rate limiting
- Access control
- Business rule evaluation
- Pattern matching

## Getting Started

Each subproject has its own documentation and examples. Start with cortex-ai for the core functionality:

```bash
# Add the core library to your project
cargo add cortex-ai

# Coming soon:
# cargo add cortex-processors
# cargo add cortex-sources
# cargo add cortex-conditions
```

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request. For major changes, please open an issue first to discuss what you would like to change.

## License

This project is licensed under the Apache 2.0 License - see the [LICENSE](LICENSE) file for details.