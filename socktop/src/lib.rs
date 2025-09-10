//! Library surface for integration tests and reuse.

pub mod types;

// Re-export connector functionality
pub use socktop_connector::{SocktopConnector, connect_to_socktop_agent};
