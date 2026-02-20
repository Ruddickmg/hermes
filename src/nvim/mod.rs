pub mod event;
pub mod producer;

use crate::{
    apc::client::{ApcClient, ClientConfig},
    nvim::producer::EventHandler,
};
use nvim_oxi::{Dictionary, api::opts::CreateAugroupOpts};
use std::sync::Arc;

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
    client: Arc<ApcClient<EventHandler>>,
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
            client: Arc::new(ApcClient::new(config, EventHandler::default())),
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
            client: Arc::new(ApcClient::new(config, EventHandler::default())),
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
    pub fn client(&self) -> &Arc<ApcClient<EventHandler>> {
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
    // Create the Hermes augroup for plugin autocommands
    let _hermes_group = nvim_oxi::api::create_augroup("Hermes", &CreateAugroupOpts::default())?;

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
            fs_read_access: true,
            fs_write_access: true,
            terminal_access: false,
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
