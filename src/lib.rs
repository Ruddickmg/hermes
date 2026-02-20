//! Hermes - APC Client for Neovim
//!
//! This library provides a clean, well-tested implementation of an Agent Client Protocol (APC)
//! client for Neovim, following clean code principles.
//!
//! # Features
//!
//! - APC client implementation using agent-client-protocol
//! - Neovim integration via nvim-oxi
//! - Clean architecture with separation of concerns
//! - Comprehensive unit and integration tests
//!
//! # Example
//!
//! ```
//! use hermes::client::{ApcClient, ClientConfig};
//!
//! let config = ClientConfig::default();
//! let client = ApcClient::new(config);
//! assert_eq!(client.config().name, "hermes");
//! ```

pub mod apc;
pub mod nvim;

// Re-export commonly used types
pub use apc::client::{ApcClient, ClientConfig};
pub use nvim::{PluginState, setup};

#[cfg(test)]
mod tests {
    use crate::nvim::producer::EventHandler;

    use super::*;

    #[test]
    fn test_library_exports() {
        let config = ClientConfig::default();
        let _client = ApcClient::new(config, EventHandler::default());
        let _state = PluginState::default();
    }
}
