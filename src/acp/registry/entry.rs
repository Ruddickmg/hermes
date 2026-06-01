use crate::acp::registry::{DistributionConfig, distribution::Distribution};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Serialize, Debug, Clone, PartialEq, Eq, Deserialize)]
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
    pub distribution: HashMap<Distribution, DistributionConfig>,
}

impl AgentEntry {
    pub fn distributions(&self) -> Vec<Distribution> {
        self.distribution.keys().cloned().collect()
    }

    pub fn get_distribution(&self, distribution: &Distribution) -> Option<&DistributionConfig> {
        self.distribution.get(distribution)
    }

    pub fn has_distribution(&self, distribution: &Distribution) -> bool {
        self.distribution.contains_key(distribution)
    }
}

impl std::hash::Hash for AgentEntry {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.id.hash(state);
        self.version.hash(state);
    }
}
