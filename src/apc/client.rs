use crate::apc::error::Error as ApcError;
use agent_client_protocol::{
    Client, ContentBlock, CreateTerminalRequest, CreateTerminalResponse, Error,
    ReadTextFileRequest, ReadTextFileResponse, ReleaseTerminalRequest, ReleaseTerminalResponse,
    RequestPermissionRequest, RequestPermissionResponse, Result, SessionNotification,
    SessionUpdate, TerminalOutputRequest, TerminalOutputResponse, WaitForTerminalExitRequest,
    WaitForTerminalExitResponse, WriteTextFileRequest, WriteTextFileResponse,
};
use nvim_oxi::Error;

#[derive(Debug, Clone)]
pub struct ClientConfig {
    pub name: String,
    pub version: String,
    pub fs_write_access: bool,
    pub fs_read_access: bool,
    pub terminal_access: bool,
}

impl Default for ClientConfig {
    fn default() -> Self {
        Self {
            name: "hermes".to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
            fs_write_access: true,
            fs_read_access: true,
            terminal_access: true,
        }
    }
}

#[derive(Clone)]
pub struct ApcClient {
    config: ClientConfig,
}

pub enum ApcEvent {
    Error(ApcError),
}

impl ApcClient {
    pub fn new(config: ClientConfig) -> Self {
        Self { config }
    }

    pub fn config(&self) -> &ClientConfig {
        &self.config
    }
}

pub fn handle_content_chunk(content: ContentBlock) -> Result<()> {
    match content {
        ContentBlock::Resource(block) => {
            println!("resource: {:?}", block.resource);
            Ok(())
        }
        ContentBlock::ResourceLink(_) => Ok(()),
        ContentBlock::Audio(_) => Err(Error::method_not_found()),
        ContentBlock::Image(_) => Ok(()),
        ContentBlock::Text(_) => Ok(()),
        _ => Err(Error::method_not_found()),
    }
}

#[async_trait::async_trait(?Send)]
impl Client for ApcClient {
    async fn request_permission(
        &self,
        _args: RequestPermissionRequest,
    ) -> Result<RequestPermissionResponse> {
        Err(Error::method_not_found())
    }

    async fn session_notification(&self, args: SessionNotification) -> Result<()> {
        match args.update {
            SessionUpdate::UserMessageChunk(chunk) => handle_content_chunk(chunk.content),
            SessionUpdate::AgentMessageChunk(chunk) => handle_content_chunk(chunk.content),
            SessionUpdate::AgentThoughtChunk(chunk) => handle_content_chunk(chunk.content),
            SessionUpdate::ToolCall(_) => Ok(()),
            SessionUpdate::ToolCallUpdate(_) => Ok(()),
            SessionUpdate::Plan(_) => Ok(()),
            SessionUpdate::AvailableCommandsUpdate(_) => Ok(()),
            SessionUpdate::CurrentModeUpdate(_) => Ok(()),
            SessionUpdate::ConfigOptionUpdate(_) => Ok(()),
            _ => Err(Error::method_not_found()),
        }
    }

    async fn write_text_file(&self, _args: WriteTextFileRequest) -> Result<WriteTextFileResponse> {
        if self.config.fs_write_access {
            Ok(WriteTextFileResponse::new())
        } else {
            Err(Error::method_not_found())
        }
    }

    async fn read_text_file(&self, _args: ReadTextFileRequest) -> Result<ReadTextFileResponse> {
        if !self.config.fs_write_access {
            return Err(Error::method_not_found());
        }
        Err(Error::method_not_found())
    }

    async fn create_terminal(
        &self,
        _args: CreateTerminalRequest,
    ) -> Result<CreateTerminalResponse> {
        if !self.config.terminal_access {
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
        if !self.config.terminal_access {
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
        if !self.config.terminal_access {
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
        if !self.config.terminal_access {
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
            fs_read_access: true,
            fs_write_access: false,
            terminal_access: true,
        };

        let client = ApcClient::new(config.clone());
        assert_eq!(client.config().name, "test-client");
        assert_eq!(client.config().version, "0.1.0");
        assert!(!client.config().fs_write_access);
        assert!(client.config().terminal_access);
    }

    #[test]
    fn test_default_config() {
        let config = ClientConfig::default();
        assert_eq!(config.name, "hermes");
        assert!(config.fs_write_access);
        assert!(config.terminal_access);
    }

    #[tokio::test]
    async fn test_session_notification() {
        use agent_client_protocol::{
            ContentBlock, ContentChunk, SessionId, SessionUpdate, TextContent,
        };

        let client = ApcClient::new(ClientConfig::default());
        let notification = SessionNotification::new(
            SessionId::new("test-session"),
            SessionUpdate::AgentMessageChunk(ContentChunk::new(ContentBlock::Text(
                TextContent::new("Hello"),
            ))),
        );

        let result = client.session_notification(notification).await;
        assert!(result.is_ok());
    }
}
