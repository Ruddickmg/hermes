use serde::Deserialize;
use std::collections::HashMap;
use std::sync::LazyLock;
use tracing::warn;

#[derive(Debug, Clone, Deserialize)]
pub struct Registry {
    pub version: String,
    pub agents: Vec<AgentEntry>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AgentEntry {
    pub id: String,
    pub name: String,
    pub version: String,
    pub description: String,
    pub repository: Option<String>,
    pub website: Option<String>,
    pub authors: Option<Vec<String>>,
    pub license: Option<String>,
    pub icon: Option<String>,
    pub distribution: Distribution,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
pub enum Distribution {
    Binary(HashMap<String, BinaryPlatformTarget>),
    Npx(PackageDistribution),
    Uvx(PackageDistribution),
}

#[derive(Debug, Clone, Deserialize)]
pub struct BinaryPlatformTarget {
    pub archive: String,
    pub cmd: String,
    pub args: Option<Vec<String>>,
    pub env: Option<HashMap<String, String>>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct PackageDistribution {
    pub package: String,
    pub args: Option<Vec<String>>,
    pub env: Option<HashMap<String, String>>,
}

impl Registry {
    pub fn bundled() -> Option<&'static Self> {
        REGISTRY.as_ref()
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
        for agent in &registry.unwrap().agents {
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
