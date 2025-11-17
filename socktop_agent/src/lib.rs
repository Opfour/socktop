//! Library interface for socktop_agent functionality
//! This allows testing of agent functions.

pub mod gpu;
pub mod metrics;
pub mod proto;
pub mod state;
pub mod tls;
pub mod types;
pub mod ws;

// Re-export commonly used types and functions for testing
pub use metrics::{collect_journal_entries, collect_process_metrics};
pub use state::{AppState, CacheEntry};
pub use types::{
    DetailedProcessInfo, JournalEntry, JournalResponse, LogLevel, ProcessMetricsResponse,
};
