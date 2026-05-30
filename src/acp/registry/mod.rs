pub mod binary;
pub mod distribution;
pub mod entry;
pub mod resolution;

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::LazyLock;
use tracing::warn;

use crate::acp::registry::entry::AgentEntry;

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct Registry {
    pub version: String,
    #[serde(deserialize_with = "deserialize_agents")]
    pub agents: HashMap<String, AgentEntry>,
}

impl Registry {
    pub fn get_entry(&self, id: &str) -> Option<&AgentEntry> {
        self.agents.get(id)
    }
}

fn deserialize_agents<'de, D>(deserializer: D) -> Result<HashMap<String, AgentEntry>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let entries = Vec::<AgentEntry>::deserialize(deserializer)?;
    Ok(entries.into_iter().map(|a| (a.id.clone(), a)).collect())
}

#[derive(Serialize, Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(untagged)]
pub enum DistributionConfig {
    BinaryTargets(HashMap<String, BinaryPlatformTarget>),
    Package(PackageDistribution),
}

#[derive(Serialize, Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct BinaryPlatformTarget {
    pub archive: String,
    pub cmd: String,
    pub args: Option<Vec<String>>,
    pub env: Option<HashMap<String, String>>,
}

#[derive(Serialize, Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct PackageDistribution {
    pub package: String,
    pub args: Option<Vec<String>>,
    pub env: Option<HashMap<String, String>>,
}

impl Registry {
    pub fn bundled() -> Option<&'static Self> {
        REGISTRY.as_ref()
    }

    pub async fn fetch(url: &str) -> crate::acp::Result<Self> {
        let url = url.to_owned();
        let text = blocking::unblock(move || -> crate::acp::Result<String> {
            let mut response = ureq::get(&url)
                .call()
                .map_err(|e| crate::acp::error::Error::Network(e.to_string()))?;
            response
                .body_mut()
                .read_to_string()
                .map_err(|e| crate::acp::error::Error::Network(e.to_string()))
        })
        .await?;
        serde_json::from_str(&text).map_err(Into::into)
    }
}

static REGISTRY: LazyLock<Option<Registry>> = LazyLock::new(|| {
    let json = include_str!("registry.json");
    serde_json::from_str(json)
        .inspect_err(|e| warn!("Failed to parse bundled registry: {e}"))
        .ok()
});

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bundled_registry_parses_successfully() {
        let registry = Registry::bundled();
        assert!(
            !registry.unwrap().version.is_empty(),
            "version should not be empty"
        );
    }

    #[test]
    fn bundled_registry_agents_have_required_fields() {
        let registry = Registry::bundled();
        // Stub has empty agents (local dev); real registry has 35+ agents (CI)
        // Both are valid — verify structure when agents exist
        for agent in registry.unwrap().agents.values() {
            assert!(!agent.id.is_empty(), "agent id should not be empty");
            assert!(!agent.name.is_empty(), "agent name should not be empty");
            assert!(
                !agent.version.is_empty(),
                "agent version should not be empty"
            );
            assert!(
                !agent.description.is_empty(),
                "agent description should not be empty"
            );
        }
    }
}
