use crate::nvim::event;
use agent_client_protocol::{
    Client, CreateTerminalRequest, CreateTerminalResponse, Error as AcpError, ReadTextFileRequest,
    ReadTextFileResponse, ReleaseTerminalRequest, ReleaseTerminalResponse,
    RequestPermissionRequest, RequestPermissionResponse, Result, SessionNotification,
    SessionUpdate, TerminalOutputRequest, TerminalOutputResponse, WaitForTerminalExitRequest,
    WaitForTerminalExitResponse, WriteTextFileRequest, WriteTextFileResponse,
};
use nvim_oxi::{Dictionary, api::opts::ExecAutocmdsOpts};

#[derive(Clone)]
pub struct EventHandler {
    group: String,
}

impl EventHandler {
    pub fn new(group: String) -> Self {
        Self { group }
    }
}

impl Default for EventHandler {
    fn default() -> Self {
        Self {
            group: "Hermes".to_string(),
        }
    }
}

#[async_trait::async_trait(?Send)]
impl Client for EventHandler {
    async fn request_permission(
        &self,
        _args: RequestPermissionRequest,
    ) -> Result<RequestPermissionResponse> {
        Err(AcpError::method_not_found())
    }

    async fn session_notification(&self, args: SessionNotification) -> Result<()> {
        let group = self.group.to_string();

        let (mut data, command) =
            match args.update {
                SessionUpdate::UserMessageChunk(chunk) => event::communication(chunk.content)
                    .map(|(dict, t)| (dict, format!("UserMessage{}", t))),
                SessionUpdate::AgentMessageChunk(chunk) => event::communication(chunk.content)
                    .map(|(dict, t)| (dict, format!("AgentMessage{}", t))),
                SessionUpdate::AgentThoughtChunk(chunk) => event::communication(chunk.content)
                    .map(|(dict, t)| (dict, format!("AgentThought{}", t))),
                SessionUpdate::ToolCall(tool_call) => event::tool_call_event(tool_call)
                    .map(|dict| (dict, "AgentToolCall".to_string())),
                SessionUpdate::ToolCallUpdate(update) => event::tool_call_update_event(update)
                    .map(|dict| (dict, "AgentToolCallUpdate".to_string())),
                SessionUpdate::Plan(plan) => {
                    event::plan_event(plan).map(|dict| (dict, "AgentPlan".to_string()))
                }
                SessionUpdate::AvailableCommandsUpdate(update) => {
                    event::available_commands_event(update)
                        .map(|dict| (dict, "AgentAvailableCommands".to_string()))
                }
                SessionUpdate::CurrentModeUpdate(update) => event::current_mode_event(update)
                    .map(|dict| (dict, "AgentCurrentMode".to_string())),
                SessionUpdate::ConfigOptionUpdate(update) => event::config_option_event(update)
                    .map(|dict| (dict, "AgentConfigOption".to_string())),
                _ => return Err(AcpError::method_not_found()),
            }?;

        data.insert("session_id", args.session_id.to_string());

        let opts = ExecAutocmdsOpts::builder().data(data).group(group).build();

        nvim_oxi::api::exec_autocmds([command.as_str()], &opts)
            .map_err(AcpError::into_internal_error)
    }

    async fn write_text_file(&self, _args: WriteTextFileRequest) -> Result<WriteTextFileResponse> {
        Err(AcpError::method_not_found())
    }

    async fn read_text_file(&self, _args: ReadTextFileRequest) -> Result<ReadTextFileResponse> {
        Err(AcpError::method_not_found())
    }

    async fn create_terminal(
        &self,
        _args: CreateTerminalRequest,
    ) -> Result<CreateTerminalResponse> {
        Err(AcpError::method_not_found())
    }

    /// Gets the terminal output and exit status
    async fn terminal_output(
        &self,
        _args: TerminalOutputRequest,
    ) -> Result<TerminalOutputResponse> {
        Err(AcpError::method_not_found())
    }

    /// Waits for a terminal command to exit
    async fn wait_for_terminal_exit(
        &self,
        _args: WaitForTerminalExitRequest,
    ) -> Result<WaitForTerminalExitResponse> {
        Err(AcpError::method_not_found())
    }

    /// Releases a terminal resource
    async fn release_terminal(
        &self,
        _args: ReleaseTerminalRequest,
    ) -> Result<ReleaseTerminalResponse> {
        Err(AcpError::method_not_found())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_session_notification() {
        use agent_client_protocol::{
            ContentBlock, ContentChunk, SessionId, SessionUpdate, TextContent,
        };

        let client = EventHandler::default();
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
