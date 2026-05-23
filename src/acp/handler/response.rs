use agent_client_protocol::schema::{
    AuthenticateResponse, ExtResponse, ForkSessionResponse, InitializeResponse,
    ListSessionsResponse, LoadSessionResponse, NewSessionResponse, PromptResponse,
    ResumeSessionResponse, SessionConfigOptionCategory, SetSessionConfigOptionResponse,
    SetSessionModeResponse, SetSessionModelResponse,
};
use tracing::instrument;

use crate::Handler;
use crate::acp::connection::Assistant;
use crate::acp::error::Error;
use crate::nvim::autocommands::Commands;

impl Handler {
    #[instrument(level = "trace", skip(self))]
    pub async fn initialized(
        &self,
        agent: &Assistant,
        info: InitializeResponse,
    ) -> Result<(), Error> {
        self.set_agent_info(agent.clone(), info.clone()).await;

        // TODO: figure out a better way to deal with the deserialization issue with the protocol version
        let value = serde_json::json!({
            "protocolVersion": info.protocol_version.to_string(),
            "agentCapabilities": {
                "loadSession": info.agent_capabilities.load_session,
                "promptCapabilities": {
                    "image": info.agent_capabilities.prompt_capabilities.image,
                    "audio": info.agent_capabilities.prompt_capabilities.audio,
                    "embeddedContext": info.agent_capabilities.prompt_capabilities.embedded_context,
                },
                "mcpCapabilities": {
                    "http": info.agent_capabilities.mcp_capabilities.http,
                    "sse": info.agent_capabilities.mcp_capabilities.sse,
                },
                "sessionCapabilities": {
                    "list": info.agent_capabilities.session_capabilities.list,
                    "fork": info.agent_capabilities.session_capabilities.fork,
                    "resume": info.agent_capabilities.session_capabilities.resume,
                },
            },
            "authMethods": info.auth_methods.iter().map(|m| serde_json::json!({
                "id": m.id().0,
                "name": m.name(),
                "description": m.description(),
            })).collect::<Vec<_>>(),
            "agentInfo": info.agent_info.map(|i| serde_json::json!({
                "name": i.name,
                "version": i.version,
                "title": i.title,
            })),
        });
        self.execute_autocommand(Commands::ConnectionInitialized, value)
            .await
    }
    #[instrument(level = "trace", skip(self))]
    pub async fn session_created(&self, session: NewSessionResponse) -> Result<(), Error> {
        let mut state = self.state.lock().await;
        state.set_session_info(&session);
        drop(state);
        self.execute_autocommand(Commands::SessionCreated, session)
            .await
    }

    #[instrument(level = "trace", skip(self))]
    pub async fn prompted(&self, response: PromptResponse) -> Result<(), Error> {
        self.execute_autocommand(Commands::Prompted, response).await
    }

    #[instrument(level = "trace", skip(self))]
    pub async fn authenticated(&self, response: AuthenticateResponse) -> Result<(), Error> {
        self.execute_autocommand(Commands::Authenticated, response)
            .await
    }

    #[instrument(level = "trace", skip(self))]
    pub async fn config_option_set(
        &self,
        response: SetSessionConfigOptionResponse,
    ) -> Result<(), Error> {
        let futures = response
            .config_options
            .iter()
            .filter_map(|c| c.category.clone())
            .map(async move |category| match category {
                SessionConfigOptionCategory::Mode => {
                    self.session_mode_set(SetSessionModeResponse::default())
                        .await
                }
                SessionConfigOptionCategory::Model => {
                    self.session_model_set(SetSessionModelResponse::default())
                        .await
                }
                _ => Ok(()),
            })
            .collect::<Vec<_>>();

        futures::future::try_join_all(futures).await?;

        self.execute_autocommand(Commands::ConfigurationUpdated, response)
            .await
    }

    #[instrument(level = "trace", skip(self))]
    pub async fn session_mode_set(&self, response: SetSessionModeResponse) -> Result<(), Error> {
        self.execute_autocommand(Commands::ModeUpdated, response)
            .await
    }

    #[instrument(level = "trace", skip(self))]
    pub async fn session_loaded(
        &self,
        session_id: String,
        response: LoadSessionResponse,
    ) -> Result<(), Error> {
        let details = response.clone();
        let mut state = self.state.lock().await;
        let session = NewSessionResponse::new(session_id)
            .modes(details.modes)
            .models(details.models)
            .config_options(details.config_options);
        state.set_session_info(&session);
        drop(state);
        self.execute_autocommand(Commands::SessionLoaded, response)
            .await
    }

    #[instrument(level = "trace", skip(self))]
    pub async fn custom_command_executed(&self, _response: ExtResponse) -> Result<(), Error> {
        Ok(())
    }

    #[instrument(level = "trace", skip(self))]
    pub async fn sessions_listed(&self, response: ListSessionsResponse) -> Result<(), Error> {
        self.execute_autocommand(Commands::SessionsListed, response)
            .await
    }

    #[instrument(level = "trace", skip(self))]
    pub async fn session_forked(&self, response: ForkSessionResponse) -> Result<(), Error> {
        self.execute_autocommand(Commands::SessionForked, response)
            .await
    }

    #[instrument(level = "trace", skip(self))]
    pub async fn session_resumed(&self, response: ResumeSessionResponse) -> Result<(), Error> {
        self.execute_autocommand(Commands::SessionResumed, response)
            .await
    }

    #[instrument(level = "trace", skip(self))]
    pub async fn session_model_set(&self, response: SetSessionModelResponse) -> Result<(), Error> {
        self.execute_autocommand(Commands::SessionModelUpdated, response)
            .await
    }
}
