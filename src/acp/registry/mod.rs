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
    use crate::acp::registry::entry::AgentEntry;
    use pretty_assertions::{assert_eq, assert_ne};

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

    #[test]
    fn registry_data_get_entry_returns_some_for_existing_agent() {
        let mut agents = HashMap::new();
        agents.insert(
            "test-agent".to_string(),
            AgentEntry {
                id: "test-agent".to_string(),
                name: "Test Agent".to_string(),
                version: "1.0.0".to_string(),
                description: "A test agent".to_string(),
                repository: None,
                website: None,
                authors: None,
                license: None,
                icon: None,
                distribution: HashMap::new(),
            },
        );
        let data = RegistryData {
            version: "1.0.0".to_string(),
            agents,
        };
        assert!(
            data.get_entry("test-agent").is_some(),
            "Should find existing agent"
        );
    }

    #[test]
    fn registry_data_get_entry_returns_none_for_missing_agent() {
        let data = RegistryData {
            version: "1.0.0".to_string(),
            agents: HashMap::new(),
        };
        assert!(
            data.get_entry("definitely-not-a-real-agent-xyz").is_none(),
            "Should return None for missing agent"
        );
    }

    #[test]
    fn registry_data_bundled_returns_some() {
        let data = RegistryData::bundled();
        assert!(
            data.is_some(),
            "Bundled registry should always be available"
        );
    }

    #[test]
    fn registry_data_bundled_contains_version() {
        let data = RegistryData::bundled().unwrap();
        assert!(
            !data.version.is_empty(),
            "Bundled registry should have a version"
        );
    }

    #[test]
    fn registry_data_equality_matches_data_only() {
        let data1 = RegistryData {
            version: "1.0.0".to_string(),
            agents: HashMap::new(),
        };
        let data2 = RegistryData {
            version: "1.0.0".to_string(),
            agents: HashMap::new(),
        };
        assert_eq!(data1, data2, "Identical data should be equal");
    }

    #[test]
    fn registry_data_inequality_detects_version_difference() {
        let data1 = RegistryData {
            version: "1.0.0".to_string(),
            agents: HashMap::new(),
        };
        let data2 = RegistryData {
            version: "2.0.0".to_string(),
            agents: HashMap::new(),
        };
        assert_ne!(data1, data2, "Different versions should not be equal");
    }
}
