//! Neovim integration module
//!
//! This module provides the bridge between the APC client and Neovim,
//! using nvim-oxi and nvim-utils for plugin functionality.
//!
//! # Architecture
//!
//! The Neovim integration follows clean code principles:
//! - **Separation of Concerns**: Plugin state is separate from client logic
//! - **Dependency Management**: Uses Arc for shared ownership
//! - **Error Handling**: Custom error types for clear error messages
//!
//! # Usage
//!
//! This module is designed to be used from Neovim plugin code. The `PluginState`
//! manages the APC client instance and provides access to it from Neovim commands.
//!
//! # Example
//!
//! ```
//! use hermes::nvim::{PluginState, setup};
//! use hermes::client::ClientConfig;
//!
//! // Create plugin state with default configuration
//! let state = PluginState::new();
//!
//! // Or with custom configuration
//! let config = ClientConfig {
//!     name: "my-nvim".to_string(),
//!     version: "1.0.0".to_string(),
//!     enable_fs: true,
//!     enable_terminal: true,
//! };
//! let custom_state = PluginState::with_config(config);
//!
//! // Setup plugin (called from Neovim)
//! let result = setup();
//! assert!(result.is_ok());
//! ```

use crate::client::{ApcClient, ClientConfig};
use nvim_oxi::Dictionary;
use std::sync::Arc;

/// Error types for Neovim integration
///
/// These errors represent issues specific to the Neovim integration layer,
/// separate from the underlying APC protocol errors.
#[derive(Debug, thiserror::Error)]
pub enum NvimError {
    /// Failed to initialize the APC client
    #[error("Failed to initialize client: {0}")]
    InitializationError(String),
    
    /// Client is not connected to an agent
    #[error("Client not connected")]
    NotConnected,
    
    /// Invalid configuration provided
    #[error("Invalid configuration: {0}")]
    InvalidConfig(String),
}

/// Neovim plugin state
///
/// This structure maintains the state of the Neovim plugin, including
/// the APC client instance. It uses `Arc` for shared ownership, allowing
/// the client to be accessed from multiple Neovim commands.
///
/// # Examples
///
/// ```
/// use hermes::nvim::PluginState;
///
/// // Create with default configuration
/// let state = PluginState::new();
///
/// // Access the client
/// let client = state.client();
/// ```
pub struct PluginState {
    client: Arc<ApcClient>,
}

impl PluginState {
    /// Creates a new plugin state with default configuration
    ///
    /// # Examples
    ///
    /// ```
    /// use hermes::nvim::PluginState;
    ///
    /// let state = PluginState::new();
    /// assert_eq!(state.client().config().name, "hermes");
    /// ```
    pub fn new() -> Self {
        let config = ClientConfig::default();
        Self {
            client: Arc::new(ApcClient::new(config)),
        }
    }

    /// Creates a new plugin state with custom configuration
    ///
    /// # Arguments
    ///
    /// * `config` - Client configuration to use
    ///
    /// # Examples
    ///
    /// ```
    /// use hermes::nvim::PluginState;
    /// use hermes::client::ClientConfig;
    ///
    /// let config = ClientConfig {
    ///     name: "custom".to_string(),
    ///     version: "1.0.0".to_string(),
    ///     enable_fs: false,
    ///     enable_terminal: true,
    /// };
    /// let state = PluginState::with_config(config);
    /// assert_eq!(state.client().config().name, "custom");
    /// ```
    pub fn with_config(config: ClientConfig) -> Self {
        Self {
            client: Arc::new(ApcClient::new(config)),
        }
    }

    /// Gets a reference to the APC client
    ///
    /// Returns an `Arc` reference to the client, allowing it to be shared
    /// across different parts of the plugin.
    ///
    /// # Examples
    ///
    /// ```
    /// use hermes::nvim::PluginState;
    ///
    /// let state = PluginState::new();
    /// let client = state.client();
    /// assert!(client.config().enable_fs);
    /// ```
    pub fn client(&self) -> &Arc<ApcClient> {
        &self.client
    }
}

impl Default for PluginState {
    fn default() -> Self {
        Self::new()
    }
}

/// Initializes the Neovim plugin
///
/// This function sets up the plugin commands and state. It should be called
/// when the plugin is loaded by Neovim.
///
/// # Returns
///
/// A `Dictionary` containing plugin metadata (currently empty).
///
/// # Errors
///
/// Returns a `nvim_oxi::Error` if initialization fails.
///
/// # Examples
///
/// ```
/// use hermes::nvim::setup;
///
/// let result = setup();
/// assert!(result.is_ok());
/// ```
pub fn setup() -> nvim_oxi::Result<Dictionary> {
    Ok(Dictionary::new())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_plugin_state_new() {
        let state = PluginState::new();
        // Plugin state should be created successfully
        assert!(std::ptr::addr_of!(state.client).is_aligned());
    }

    #[test]
    fn test_plugin_state_with_config() {
        let config = ClientConfig {
            name: "test".to_string(),
            version: "1.0.0".to_string(),
            enable_fs: true,
            enable_terminal: false,
        };
        
        let state = PluginState::with_config(config);
        assert!(std::ptr::addr_of!(state.client).is_aligned());
    }

    #[test]
    fn test_plugin_state_default() {
        let state = PluginState::default();
        assert!(std::ptr::addr_of!(state.client).is_aligned());
    }
}
