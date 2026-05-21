use std::collections::HashMap;

use crate::{
    acp::{Result, connection::Assistant, error::Error},
    nvim::{configuration::ClientConfig, state::agent::AgentInfo},
};
use agent_client_protocol::schema::InitializeResponse;
use tracing::{debug, instrument};

pub mod agent;

#[derive(Debug)]
pub struct PluginState {
    pub config: ClientConfig,
    pub prompt: HashMap<String, String>,
    pub agent_info: AgentInfo,
}

impl PluginState {
    #[instrument(level = "trace")]
    pub fn new() -> Self {
        Self::with_config(ClientConfig::default())
    }

    #[instrument(level = "trace")]
    pub fn with_config(config: ClientConfig) -> Self {
        Self {
            config,
            prompt: HashMap::new(),
            agent_info: AgentInfo::default(),
        }
    }

    #[instrument(level = "trace")]
    pub fn update_session_prompt_id(&mut self, session_id: String, prompt_id: String) -> &mut Self {
        self.prompt.insert(session_id.clone(), prompt_id.clone());
        debug!("Updated prompt for session '{}'", session_id);
        self
    }

    #[instrument(level = "trace")]
    pub fn get_session_prompt(&self, session_id: &str) -> Result<String> {
        self.prompt
            .get(session_id)
            .cloned()
            .ok_or(Error::Internal("Prompt id was not initialized".to_string()))
    }

    #[instrument(level = "trace")]
    pub fn set_agent(&mut self, agent: Assistant) -> &mut Self {
        self.agent_info.set_agent(agent.clone());
        debug!("Updated current agent to: '{}'", agent);
        self
    }

    #[instrument(level = "trace")]
    pub fn set_agent_info(&mut self, agent: Assistant, info: InitializeResponse) -> &mut Self {
        self.agent_info.add_agent(agent.clone(), info.clone());
        debug!("Updated information for '{}': {:#?}", agent, info);
        self
    }
}

impl Default for PluginState {
    fn default() -> Self {
        Self::new()
    }
}
