use nvim_oxi::conversion::FromObject;
use serde::Serialize;
use tracing::warn;

use crate::{
    acp::registry::{AgentEntry, Distribution, Registry},
    api::Api,
    nvim::autocommands::Commands,
};

const DEFAULT_REGISTRY_URL: &str =
    "https://cdn.agentclientprotocol.com/registry/v1/latest/registry.json";

#[derive(Debug, Clone, Default)]
pub struct AgentsConfig {
    pub update: Option<bool>,
    pub url: Option<String>,
}

impl nvim_oxi::conversion::FromObject for AgentsConfig {
    fn from_object(obj: nvim_oxi::Object) -> Result<Self, nvim_oxi::conversion::Error> {
        if obj.is_nil() {
            return Ok(Self::default());
        }

        let dict: nvim_oxi::Dictionary = obj.try_into()?;

        let update: Option<bool> = dict
            .get("update")
            .map(|o| bool::from_object(o.clone()))
            .transpose()?;

        let url: Option<String> = dict.get("url").and_then(|o| {
            TryInto::<nvim_oxi::String>::try_into(o.clone())
                .ok()
                .map(|s: nvim_oxi::String| s.to_string())
        });

        Ok(Self { update, url })
    }
}

impl nvim_oxi::lua::Poppable for AgentsConfig {
    unsafe fn pop(lua_state: *mut nvim_oxi::lua::ffi::State) -> Result<Self, nvim_oxi::lua::Error> {
        let obj = unsafe { nvim_oxi::Object::pop(lua_state)? };
        Ok(Self::from_object(obj)
            .inspect_err(|e| tracing::error!("{:?}", e))
            .unwrap_or_default())
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct AgentListEntry {
    pub id: String,
    pub name: String,
    pub version: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub license: Option<String>,
    pub description: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub website: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub repository: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub icon: Option<String>,
    pub distributions: Vec<String>,
}

#[derive(Debug, Serialize)]
struct AgentListPayload {
    agents: Vec<AgentListEntry>,
}

impl From<AgentEntry> for AgentListEntry {
    fn from(entry: AgentEntry) -> Self {
        let distribution_label = match entry.distribution {
            Distribution::Binary(_) => "binary",
            Distribution::Npx(_) => "npx",
            Distribution::Uvx(_) => "uvx",
        };

        Self {
            distributions: vec![distribution_label.to_string()],
            id: entry.id,
            name: entry.name,
            version: entry.version,
            license: entry.license,
            description: entry.description,
            website: entry.website,
            repository: entry.repository,
            icon: entry.icon,
        }
    }
}

impl Api {
    #[tracing::instrument(level = "trace", skip(self))]
    pub async fn agents(&self, maybe_config: Option<AgentsConfig>) -> crate::acp::Result<()> {
        let config = maybe_config.unwrap_or_default();
        let mut state = self.state.lock().await;

        if config.update.unwrap_or(false) {
            let url = config.url.as_deref().unwrap_or(DEFAULT_REGISTRY_URL);
            match Registry::fetch(url).await {
                Ok(fetched) => {
                    state.registry = Some(fetched);
                }
                Err(e) => {
                    if state.registry.is_some() {
                        warn!("Failed to update agent registry (keeping existing): {}", e);
                    } else {
                        return Err(e);
                    }
                }
            }
        }

        let agents: Vec<AgentListEntry> = state
            .registry
            .as_ref()
            .map(|r| r.agents.iter().cloned().map(Into::into).collect())
            .unwrap_or_default();

        drop(state);

        let _ = self
            .response_handler
            .execute_autocommand(Commands::AgentList, AgentListPayload { agents })
            .await;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nvim_oxi::{Dictionary, Object};
    use pretty_assertions::assert_eq;
    use std::collections::HashMap;

    #[test]
    fn test_agents_config_from_object_nil() {
        let config = AgentsConfig::from_object(Object::nil()).unwrap();
        assert!(
            config.update.is_none() && config.url.is_none(),
            "Nil should produce default config"
        );
    }

    #[test]
    fn test_agents_config_from_object_empty_dict() {
        let config = AgentsConfig::from_object(Object::from(Dictionary::new())).unwrap();
        assert!(
            config.update.is_none() && config.url.is_none(),
            "Empty dict should produce default config"
        );
    }

    #[test]
    fn test_agents_config_from_object_update_true() {
        let mut dict = Dictionary::new();
        dict.insert("update", true);
        let config = AgentsConfig::from_object(Object::from(dict)).unwrap();
        assert_eq!(config.update, Some(true));
    }

    #[test]
    fn test_agents_config_from_object_update_false() {
        let mut dict = Dictionary::new();
        dict.insert("update", false);
        let config = AgentsConfig::from_object(Object::from(dict)).unwrap();
        assert_eq!(config.update, Some(false));
    }

    #[test]
    fn test_agents_config_from_object_url() {
        let mut dict = Dictionary::new();
        dict.insert("url", "https://example.com/registry.json");
        let config = AgentsConfig::from_object(Object::from(dict)).unwrap();
        assert_eq!(
            config.url,
            Some("https://example.com/registry.json".to_string())
        );
    }

    #[test]
    fn test_agents_config_from_object_update_and_url() {
        let mut dict = Dictionary::new();
        dict.insert("update", true);
        dict.insert("url", "https://example.com/registry.json");
        let config = AgentsConfig::from_object(Object::from(dict)).unwrap();
        assert_eq!(config.update, Some(true));
        assert_eq!(
            config.url,
            Some("https://example.com/registry.json".to_string())
        );
    }

    #[test]
    fn test_agents_config_from_object_unknown_fields_ignored() {
        let mut dict = Dictionary::new();
        dict.insert("unknown", "value");
        dict.insert("update", true);
        let config = AgentsConfig::from_object(Object::from(dict)).unwrap();
        assert_eq!(config.update, Some(true));
    }

    #[test]
    fn test_agents_config_from_object_invalid_type_errors() {
        let arr = nvim_oxi::Array::from_iter(vec![Object::from("test")]);
        let result = AgentsConfig::from_object(Object::from(arr));
        assert!(result.is_err(), "Array should not be a valid AgentsConfig");
    }

    #[test]
    fn test_agent_list_entry_from_binary_distribution() {
        let entry = AgentEntry {
            id: "test-agent".to_string(),
            name: "Test Agent".to_string(),
            version: "1.0.0".to_string(),
            description: "A test agent".to_string(),
            repository: Some("https://github.com/test/repo".to_string()),
            website: Some("https://test.dev".to_string()),
            authors: Some(vec!["test".to_string()]),
            license: Some("MIT".to_string()),
            icon: Some("https://test.dev/icon.png".to_string()),
            distribution: Distribution::Binary(HashMap::new()),
        };

        let list_entry = AgentListEntry::from(entry);
        assert_eq!(list_entry.id, "test-agent");
        assert_eq!(list_entry.name, "Test Agent");
        assert_eq!(list_entry.version, "1.0.0");
        assert_eq!(list_entry.description, "A test agent");
        assert_eq!(
            list_entry.repository,
            Some("https://github.com/test/repo".to_string())
        );
        assert_eq!(list_entry.website, Some("https://test.dev".to_string()));
        assert_eq!(list_entry.license, Some("MIT".to_string()));
        assert_eq!(
            list_entry.icon,
            Some("https://test.dev/icon.png".to_string())
        );
        assert_eq!(list_entry.distributions, vec!["binary"]);
    }

    #[test]
    fn test_agent_list_entry_from_npx_distribution() {
        let entry = AgentEntry {
            id: "npx-agent".to_string(),
            name: "NPX Agent".to_string(),
            version: "2.0.0".to_string(),
            description: "An NPX agent".to_string(),
            repository: None,
            website: None,
            authors: None,
            license: None,
            icon: None,
            distribution: Distribution::Npx(crate::acp::registry::PackageDistribution {
                package: "npx-pkg".to_string(),
                args: None,
                env: None,
            }),
        };

        let list_entry = AgentListEntry::from(entry);
        assert_eq!(list_entry.id, "npx-agent");
        assert_eq!(list_entry.name, "NPX Agent");
        assert!(list_entry.repository.is_none());
        assert!(list_entry.website.is_none());
        assert!(list_entry.license.is_none());
        assert!(list_entry.icon.is_none());
        assert_eq!(list_entry.distributions, vec!["npx"]);
    }

    #[test]
    fn test_agent_list_entry_from_uvx_distribution() {
        let entry = AgentEntry {
            id: "uvx-agent".to_string(),
            name: "UVX Agent".to_string(),
            version: "3.0.0".to_string(),
            description: "A UVX agent".to_string(),
            repository: Some("https://github.com/uvx/repo".to_string()),
            website: None,
            authors: Some(vec!["author".to_string()]),
            license: Some("Apache-2.0".to_string()),
            icon: None,
            distribution: Distribution::Uvx(crate::acp::registry::PackageDistribution {
                package: "uvx-pkg".to_string(),
                args: None,
                env: None,
            }),
        };

        let list_entry = AgentListEntry::from(entry);
        assert_eq!(list_entry.id, "uvx-agent");
        assert_eq!(list_entry.distributions, vec!["uvx"]);
    }

    #[test]
    fn test_agent_list_payload_serialization() {
        let entry = AgentEntry {
            id: "test".to_string(),
            name: "Test".to_string(),
            version: "1.0".to_string(),
            description: "desc".to_string(),
            repository: None,
            website: None,
            authors: None,
            license: Some("MIT".to_string()),
            icon: None,
            distribution: Distribution::Binary(HashMap::new()),
        };
        let payload = AgentListPayload {
            agents: vec![AgentListEntry::from(entry)],
        };
        let json = serde_json::to_value(payload).unwrap();
        assert_eq!(json["agents"][0]["id"], "test");
        assert_eq!(json["agents"][0]["distributions"][0], "binary");
        assert_eq!(json["agents"][0].get("license").unwrap(), "MIT");
        assert_eq!(json["agents"][0]["description"], "desc");
        assert_eq!(json["agents"][0]["name"], "Test");
        assert_eq!(json["agents"][0]["version"], "1.0");
    }
}
