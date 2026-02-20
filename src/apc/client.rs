use agent_client_protocol::{
    Client, CreateTerminalRequest, CreateTerminalResponse, Error as AcpError, ReadTextFileRequest,
    ReadTextFileResponse, ReleaseTerminalRequest, ReleaseTerminalResponse,
    RequestPermissionRequest, RequestPermissionResponse, Result, SessionNotification,
    TerminalOutputRequest, TerminalOutputResponse, WaitForTerminalExitRequest,
    WaitForTerminalExitResponse, WriteTextFileRequest, WriteTextFileResponse,
};

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
pub struct ApcClient<H: Client> {
    config: ClientConfig,
    handler: H,
}

impl<H: Client> ApcClient<H> {
    pub fn new(config: ClientConfig, handler: H) -> Self {
        Self { config, handler }
    }

    pub fn config(&self) -> &ClientConfig {
        &self.config
    }
}

#[async_trait::async_trait(?Send)]
impl<H: Client> Client for ApcClient<H> {
    async fn request_permission(
        &self,
        args: RequestPermissionRequest,
    ) -> Result<RequestPermissionResponse> {
        self.handler.request_permission(args).await
    }

    async fn session_notification(&self, args: SessionNotification) -> Result<()> {
        self.handler.session_notification(args).await
    }

    async fn write_text_file(&self, args: WriteTextFileRequest) -> Result<WriteTextFileResponse> {
        if self.config.fs_write_access {
            self.handler.write_text_file(args).await?;
            Ok(WriteTextFileResponse::new())
        } else {
            Err(AcpError::method_not_found())
        }
    }

    async fn read_text_file(&self, _args: ReadTextFileRequest) -> Result<ReadTextFileResponse> {
        if self.config.fs_read_access {
            self.handler.read_text_file(_args).await
        } else {
            Err(AcpError::method_not_found())
        }
    }

    async fn create_terminal(&self, args: CreateTerminalRequest) -> Result<CreateTerminalResponse> {
        if self.config.terminal_access {
            self.create_terminal(args).await
        } else {
            Err(AcpError::method_not_found())
        }
    }

    async fn terminal_output(&self, args: TerminalOutputRequest) -> Result<TerminalOutputResponse> {
        if self.config.terminal_access {
            self.handler.terminal_output(args).await
        } else {
            Err(AcpError::method_not_found())
        }
    }

    async fn wait_for_terminal_exit(
        &self,
        args: WaitForTerminalExitRequest,
    ) -> Result<WaitForTerminalExitResponse> {
        if self.config.terminal_access {
            self.handler.wait_for_terminal_exit(args).await
        } else {
            Err(AcpError::method_not_found())
        }
    }

    async fn release_terminal(
        &self,
        args: ReleaseTerminalRequest,
    ) -> Result<ReleaseTerminalResponse> {
        if self.config.terminal_access {
            self.handler.release_terminal(args).await
        } else {
            Err(AcpError::method_not_found())
        }
    }
}

// #[cfg(test)]
// mod tests {
//     use super::*;
//
//     #[test]
//     fn test_new_client() {
//         let config = ClientConfig::default();
//         let client = ApcClient::new(config);
//         assert_eq!(client.config().name, "hermes");
//     }
//
//     #[test]
//     fn test_custom_config() {
//         let config = ClientConfig {
//             name: "test-client".to_string(),
//             version: "0.1.0".to_string(),
//             fs_read_access: true,
//             fs_write_access: false,
//             terminal_access: true,
//         };
//
//         let client = ApcClient::new(config.clone());
//         assert_eq!(client.config().name, "test-client");
//         assert_eq!(client.config().version, "0.1.0");
//         assert!(!client.config().fs_write_access);
//         assert!(client.config().terminal_access);
//     }
//
//     #[test]
//     fn test_default_config() {
//         let config = ClientConfig::default();
//         assert_eq!(config.name, "hermes");
//         assert!(config.fs_write_access);
//         assert!(config.terminal_access);
//     }
//
//     #[tokio::test]
//     async fn test_session_notification() {
//         use agent_client_protocol::{
//             ContentBlock, ContentChunk, SessionId, SessionUpdate, TextContent,
//         };
//
//         let client = ApcClient::new(ClientConfig::default());
//         let notification = SessionNotification::new(
//             SessionId::new("test-session"),
//             SessionUpdate::AgentMessageChunk(ContentChunk::new(ContentBlock::Text(
//                 TextContent::new("Hello"),
//             ))),
//         );
//
//         let result = client.session_notification(notification).await;
//         assert!(result.is_ok());
//     }
// }
