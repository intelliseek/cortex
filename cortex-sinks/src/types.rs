//! Common types used across different sinks

use std::path::PathBuf;

/// Configuration for file system sinks
#[derive(Debug, Clone)]
pub struct FileSinkConfig {
    /// Path to write output to
    pub path: PathBuf,
    /// Whether to append to existing file
    pub append: bool,
}

/// Configuration for database sinks
#[derive(Debug, Clone)]
pub struct DatabaseSinkConfig {
    /// Database connection string
    pub connection_string: String,
    /// Table or collection name
    pub table_name: String,
}

/// Configuration for message queue sinks
#[derive(Debug, Clone)]
pub struct QueueSinkConfig {
    /// Queue connection string
    pub connection_string: String,
    /// Queue or topic name
    pub queue_name: String,
}

/// Configuration for metrics sinks
#[derive(Debug, Clone)]
pub struct MetricsSinkConfig {
    /// Metrics endpoint
    pub endpoint: String,
    /// Metric name
    pub metric_name: String,
}

/// Configuration for vector store sinks
#[derive(Debug, Clone)]
pub struct VectorStoreSinkConfig {
    /// Vector store connection string
    pub connection_string: String,
    /// Collection name
    pub collection_name: String,
    /// Vector dimension
    pub dimension: usize,
} 