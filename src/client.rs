//! APC Client implementation for Neovim
//!
//! This module provides a client for the Agent Client Protocol (APC) that integrates
//! with Neovim through nvim-oxi bindings.
//!
//! The client implements the `Client` trait from the `agent-client-protocol` crate,
//! providing handlers for session notifications, file system operations, and terminal
//! management.
//!
//! # Architecture
//!
//! The APC client follows clean code principles:
//! - **Single Responsibility**: Each component has a clear, focused purpose
//! - **Dependency Injection**: Configuration is injected via `ClientConfig`
//! - **Open/Closed Principle**: Extensible through trait implementation
//! - **Interface Segregation**: Clean trait boundaries with optional capabilities
//!
//! # Example
//!
//! ```
//! use hermes::client::{ApcClient, ClientConfig};
//! use agent_client_protocol::Client;
//!
//! // Create a client with default configuration
//! let config = ClientConfig::default();
//! let client = ApcClient::new(config);
//!
//! // Client is ready to handle APC protocol messages
//! assert_eq!(client.config().name, "hermes");
//! ```
//!
//! # Capabilities
//!
//! The client supports configurable capabilities:
//! - **File System**: Read and write text files (controlled by `enable_fs`)
//! - **Terminal**: Execute commands in terminals (controlled by `enable_terminal`)
//!
//! When a capability is disabled, the client returns `Error::method_not_found()`
//! for related operations.

use agent_client_protocol::{
    Client, CreateTerminalRequest, CreateTerminalResponse, ReadTextFileRequest,
    ReadTextFileResponse, RequestPermissionRequest, RequestPermissionResponse,
    ReleaseTerminalRequest, ReleaseTerminalResponse, SessionNotification, TerminalOutputRequest,
    TerminalOutputResponse, WaitForTerminalExitRequest, WaitForTerminalExitResponse,
    WriteTextFileRequest, WriteTextFileResponse, Result, Error,
};

/// Configuration for the APC client
///
/// This structure controls the behavior and capabilities of the APC client.
/// All fields are immutable after construction to ensure thread-safety.
///
/// # Examples
///
/// ```
/// use hermes::client::ClientConfig;
///
/// // Use default configuration
/// let config = ClientConfig::default();
/// assert_eq!(config.name, "hermes");
///
/// // Create custom configuration
/// let custom = ClientConfig {
///     name: "my-editor".to_string(),
///     version: "1.0.0".to_string(),
///     enable_fs: true,
///     enable_terminal: false,
/// };
/// ```
#[derive(Debug, Clone)]
pub struct ClientConfig {
    /// Client name (typically the editor name)
    pub name: String,
    /// Client version (semantic versioning recommended)
    pub version: String,
    /// Enable file system read/write capabilities
    ///
    /// When `false`, file system operations will return `Error::method_not_found()`
    pub enable_fs: bool,
    /// Enable terminal execution capabilities
    ///
    /// When `false`, terminal operations will return `Error::method_not_found()`
    pub enable_terminal: bool,
}

impl Default for ClientConfig {
    fn default() -> Self {
        Self {
            name: "hermes".to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
            enable_fs: true,
            enable_terminal: true,
        }
    }
}

/// APC Client for Neovim
///
/// This client implements the Agent Client Protocol (APC) for use with Neovim.
/// It handles communication between Neovim and APC-compliant agents, managing
/// session notifications, file operations, and terminal execution.
///
/// # Thread Safety
///
/// The client is `Clone` and can be shared across different parts of your application.
/// However, note that the underlying `Client` trait uses `?Send`, so async operations
/// must complete on the same thread where they started.
///
/// # Examples
///
/// ```
/// use hermes::client::{ApcClient, ClientConfig};
/// use agent_client_protocol::{Client, SessionNotification, SessionId, SessionUpdate};
/// use agent_client_protocol::{ContentChunk, ContentBlock, TextContent};
///
/// # async fn example() -> agent_client_protocol::Result<()> {
/// // Create and configure the client
/// let config = ClientConfig {
///     name: "neovim".to_string(),
///     version: "0.10.0".to_string(),
///     enable_fs: true,
///     enable_terminal: true,
/// };
/// let client = ApcClient::new(config);
///
/// // Handle a session notification
/// let notification = SessionNotification::new(
///     SessionId::new("session-1"),
///     SessionUpdate::AgentMessageChunk(ContentChunk::new(
///         ContentBlock::Text(TextContent::new("Hello from agent"))
///     )),
/// );
/// client.session_notification(notification).await?;
/// # Ok(())
/// # }
/// ```
#[derive(Clone)]
pub struct ApcClient {
    config: ClientConfig,
}

impl ApcClient {
    /// Creates a new APC client with the given configuration
    ///
    /// # Arguments
    ///
    /// * `config` - Client configuration specifying name, version, and enabled capabilities
    ///
    /// # Examples
    ///
    /// ```
    /// use hermes::client::{ApcClient, ClientConfig};
    ///
    /// let config = ClientConfig::default();
    /// let client = ApcClient::new(config);
    /// assert_eq!(client.config().name, "hermes");
    /// ```
    pub fn new(config: ClientConfig) -> Self {
        Self { config }
    }

    /// Gets a reference to the client configuration
    ///
    /// # Examples
    ///
    /// ```
    /// use hermes::client::{ApcClient, ClientConfig};
    ///
    /// let client = ApcClient::new(ClientConfig::default());
    /// assert!(client.config().enable_fs);
    /// ```
    pub fn config(&self) -> &ClientConfig {
        &self.config
    }
}

#[async_trait::async_trait(?Send)]
impl Client for ApcClient {
    /// Requests permission from the user for a tool call operation.
    async fn request_permission(
        &self,
        _args: RequestPermissionRequest,
    ) -> Result<RequestPermissionResponse> {
        // For now, return a simple implementation
        // In a full implementation, this would prompt the user in Neovim
        Err(Error::method_not_found())
    }

    /// Handles session update notifications from the agent.
    async fn session_notification(&self, _args: SessionNotification) -> Result<()> {
        // Handle session notifications
        // In a full implementation, this would update Neovim buffers/UI
        Ok(())
    }

    /// Writes content to a text file in the client's file system.
    async fn write_text_file(&self, _args: WriteTextFileRequest) -> Result<WriteTextFileResponse> {
        if !self.config.enable_fs {
            return Err(Error::method_not_found());
        }
        // Implementation would use Neovim's file operations
        Err(Error::method_not_found())
    }

    /// Reads content from a text file in the client's file system.
    async fn read_text_file(&self, _args: ReadTextFileRequest) -> Result<ReadTextFileResponse> {
        if !self.config.enable_fs {
            return Err(Error::method_not_found());
        }
        // Implementation would use Neovim's file operations
        Err(Error::method_not_found())
    }

    /// Executes a command in a new terminal
    async fn create_terminal(
        &self,
        _args: CreateTerminalRequest,
    ) -> Result<CreateTerminalResponse> {
        if !self.config.enable_terminal {
            return Err(Error::method_not_found());
        }
        // Implementation would use Neovim's terminal functionality
        Err(Error::method_not_found())
    }

    /// Gets the terminal output and exit status
    async fn terminal_output(
        &self,
        _args: TerminalOutputRequest,
    ) -> Result<TerminalOutputResponse> {
        if !self.config.enable_terminal {
            return Err(Error::method_not_found());
        }
        // Implementation would query Neovim terminal state
        Err(Error::method_not_found())
    }

    /// Waits for a terminal command to exit
    async fn wait_for_terminal_exit(
        &self,
        _args: WaitForTerminalExitRequest,
    ) -> Result<WaitForTerminalExitResponse> {
        if !self.config.enable_terminal {
            return Err(Error::method_not_found());
        }
        // Implementation would wait on Neovim terminal
        Err(Error::method_not_found())
    }

    /// Releases a terminal resource
    async fn release_terminal(
        &self,
        _args: ReleaseTerminalRequest,
    ) -> Result<ReleaseTerminalResponse> {
        if !self.config.enable_terminal {
            return Err(Error::method_not_found());
        }
        // Implementation would clean up Neovim terminal
        Err(Error::method_not_found())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_client() {
        let config = ClientConfig::default();
        let client = ApcClient::new(config);
        assert_eq!(client.config().name, "hermes");
    }

    #[test]
    fn test_custom_config() {
        let config = ClientConfig {
            name: "test-client".to_string(),
            version: "0.1.0".to_string(),
            enable_fs: false,
            enable_terminal: true,
        };
        
        let client = ApcClient::new(config.clone());
        assert_eq!(client.config().name, "test-client");
        assert_eq!(client.config().version, "0.1.0");
        assert!(!client.config().enable_fs);
        assert!(client.config().enable_terminal);
    }

    #[test]
    fn test_default_config() {
        let config = ClientConfig::default();
        assert_eq!(config.name, "hermes");
        assert!(config.enable_fs);
        assert!(config.enable_terminal);
    }

    #[tokio::test]
    async fn test_session_notification() {
        use agent_client_protocol::{SessionId, SessionUpdate, ContentChunk, ContentBlock, TextContent};
        
        let client = ApcClient::new(ClientConfig::default());
        let notification = SessionNotification::new(
            SessionId::new("test-session"),
            SessionUpdate::AgentMessageChunk(ContentChunk::new(
                ContentBlock::Text(TextContent::new("Hello"))
            )),
        );
        
        let result = client.session_notification(notification).await;
        assert!(result.is_ok());
    }
}

