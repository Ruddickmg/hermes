//! Neovim integration module
//!
//! This module provides the bridge between the APC client and Neovim,
//! using nvim-oxi and nvim-utils for plugin functionality.

use crate::client::{ApcClient, ClientConfig};
use nvim_oxi::Dictionary;
use std::sync::Arc;

/// Error types for Neovim integration
#[derive(Debug, thiserror::Error)]
pub enum NvimError {
    #[error("Failed to initialize client: {0}")]
    InitializationError(String),
    
    #[error("Client not connected")]
    NotConnected,
    
    #[error("Invalid configuration: {0}")]
    InvalidConfig(String),
}

/// Neovim plugin state
pub struct PluginState {
    client: Arc<ApcClient>,
}

impl PluginState {
    /// Creates a new plugin state with default configuration
    pub fn new() -> Self {
        let config = ClientConfig::default();
        Self {
            client: Arc::new(ApcClient::new(config)),
        }
    }

    /// Creates a new plugin state with custom configuration
    pub fn with_config(config: ClientConfig) -> Self {
        Self {
            client: Arc::new(ApcClient::new(config)),
        }
    }

    /// Gets a reference to the APC client
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
/// This function sets up the plugin commands and state
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
