pub mod binary;
pub mod distribution;
pub mod entry;
pub mod resolution;

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::LazyLock;
use tracing::warn;

use crate::acp::registry::entry::AgentEntry;
use crate::utilities::downloader::Downloader;

/// Pure data object deserialized from the registry JSON.
/// Can be serialized, cloned, and passed around freely.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct RegistryData {
    pub version: String,
    #[serde(deserialize_with = "deserialize_agents")]
    pub agents: HashMap<String, AgentEntry>,
}

impl RegistryData {
    pub fn get_entry(&self, id: &str) -> Option<&AgentEntry> {
        self.agents.get(id)
    }

    pub fn bundled() -> Option<RegistryData> {
        REGISTRY_DATA.as_ref().map(|d| RegistryData {
            version: d.version.clone(),
            agents: d.agents.clone(),
        })
    }
}

/// Functional wrapper that holds registry data plus runtime utilities
/// like the notification messenger for progress reporting.
#[derive(Clone, Debug)]
pub struct Registry {
    pub data: RegistryData,
    downloader: Downloader,
}

impl PartialEq for Registry {
    fn eq(&self, other: &Self) -> bool {
        self.data == other.data
    }
}

impl Eq for Registry {}

impl std::hash::Hash for Registry {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.data.hash(state);
    }
}

impl Registry {
    pub fn new(downloader: Downloader) -> Option<Self> {
        RegistryData::bundled().map(|data| Self { data, downloader })
    }

    pub async fn fetch(&self, url: &str) -> crate::acp::Result<Registry> {
        let url = url.to_owned();
        let downloader = self.downloader.clone();
        let text = blocking::unblock(move || {
            downloader.download_to_string(&url, "hermes-registry-update", "Updating agent registry")
        })
        .await?;

        Ok(Self {
            data: serde_json::from_str(&text)?,
            downloader: self.downloader.clone(),
        })
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
pub enum DistributionCommand {
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

impl std::hash::Hash for RegistryData {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.version.hash(state);
        for (key, agent) in &self.agents {
            key.hash(state);
            agent.hash(state);
        }
    }
}

static REGISTRY_DATA: LazyLock<Option<RegistryData>> = LazyLock::new(|| {
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
        let data = REGISTRY_DATA.as_ref().unwrap();
        assert!(!data.version.is_empty(), "version should not be empty");
    }

    #[test]
    fn bundled_registry_agents_have_required_fields() {
        let data = REGISTRY_DATA.as_ref().unwrap();
        // Stub has empty agents (local dev); real registry has 35+ agents (CI)
        // Both are valid — verify structure when agents exist
        for agent in data.agents.values() {
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
