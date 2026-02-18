pub mod copilot;

use agent_client_protocol::AgentCapabilities;

pub use copilot::*;

#[derive(Debug, Clone)]
pub struct AgentConfig {
    pub name: String,
    pub version: String,
    pub capabilities: AgentCapabilities,
}

impl Default for AgentConfig {
    fn default() -> Self {
        Self {
            name: "hermes-agent".to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
            capabilities: AgentCapabilities::default(),
        }
    }
}
