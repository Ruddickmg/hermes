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
    use super::nvim::{ConnectionArgs, ConnectionDetails};
    use crate::apc::connection::{Assistant, Protocol};

    #[test]
    fn test_connection_args_to_connection_details() {
        let args = ConnectionArgs {
            agent: Some(Assistant::Copilot),
            protocol: Some(Protocol::Stdio),
        };
        let details = ConnectionDetails::from(args);
        assert_eq!(details.agent, Assistant::Copilot);
        assert_eq!(details.protocol, Protocol::Stdio);
    }

    #[test]
    fn test_connection_args_defaults() {
        let args = ConnectionArgs {
            agent: None,
            protocol: None,
        };
        let details = ConnectionDetails::from(args);
        assert_eq!(details.agent, Assistant::Copilot);
        assert_eq!(details.protocol, Protocol::Stdio);
    }
}
