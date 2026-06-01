use crate::{
    acp::{
        Result,
        connection::Assistant,
        error::Error,
        registry::{DistributionCommand, PackageDistribution, binary, distribution::Distribution},
        utilities::command::command_available,
    },
    nvim::configuration::DistributionsConfig,
};

fn make_package_assistant(agent_id: &str, command: &str, pkg: &PackageDistribution) -> Assistant {
    let mut args = vec![pkg.package.clone()];
    if let Some(dist_args) = &pkg.args {
        args.extend(dist_args.clone());
    }
    Assistant::CustomStdio {
        name: agent_id.to_string(),
        command: command.to_string(),
        args,
    }
}

pub async fn fetch_agent_from_registry(
    entry: &crate::acp::registry::AgentEntry,
    preference: Option<Distribution>,
    distributions_config: &DistributionsConfig,
) -> Result<Assistant> {
    let agent_id = &entry.id;

    if let Some(pref) = preference {
        if !pref.is_enabled(distributions_config) {
            return Err(Error::InvalidInput(format!(
                "Distribution '{}' was selected but is disabled",
                pref
            )));
        }
    }

    let mut considered_distributions: Vec<Distribution> = preference
        .map(|p| vec![p])
        .unwrap_or_else(|| entry.distributions());
    considered_distributions.retain(|distribution| entry.has_distribution(distribution));

    // Ensure deterministic auto-selection order when no explicit preference is provided.
    if preference.is_none() {
        considered_distributions.sort_by_key(|d| match d {
            Distribution::Npx => 0,
            Distribution::Uvx => 1,
            Distribution::Binary => 2,
            Distribution::Invalid => 3,
        });
    }

    let candidates: Vec<Distribution> = considered_distributions
        .iter()
        .copied()
        .filter(|distribution| distribution.is_enabled(distributions_config))
        .collect();

    for dist_type in &candidates {
        match entry.get_distribution(dist_type) {
            Some(DistributionCommand::Package(pkg)) => {
                if command_available(&dist_type.to_string()).await {
                    return Ok(make_package_assistant(
                        agent_id,
                        &dist_type.to_string(),
                        pkg,
                    ));
                }
            }
            Some(DistributionCommand::BinaryTargets(targets)) => {
                if let Some(target) = binary::platform_target(targets) {
                    let cache_dir_override = if distributions_config.binary.path.is_empty() {
                        None
                    } else {
                        Some(distributions_config.binary.path.as_str())
                    };
                    let path = binary::get_binary(
                        agent_id,
                        &entry.version,
                        target,
                        cache_dir_override.map(std::path::Path::new),
                    )
                    .await?;
                    return Ok(Assistant::CustomStdio {
                        name: agent_id.to_string(),
                        command: path.to_string_lossy().to_string(),
                        args: target.args.clone().unwrap_or_default(),
                    });
                }
            }
            _ => {}
        }
    }

    if candidates.is_empty() && !considered_distributions.is_empty() {
        Err(Error::InvalidInput(format!(
            "Agent '{}' has all distribution methods disabled. Set at least one of \
             'distributions.uvx', 'distributions.npx', or 'distributions.binary.enabled' to true.",
            entry.id
        )))
    } else {
        Err(Error::InvalidInput(format!(
            "Agent '{}' has no supported distribution for this platform",
            entry.id
        )))
    }
}

#[cfg(test)]
mod tests {
    use std::{collections::HashMap, sync::Arc};

    use async_lock::Mutex;

    use crate::PluginState;

    use super::*;

    fn entry_with_distribution(
        id: &str,
        distribution: HashMap<Distribution, DistributionCommand>,
    ) -> crate::acp::registry::AgentEntry {
        crate::acp::registry::AgentEntry {
            id: id.to_string(),
            name: id.to_string(),
            version: "1.0.0".into(),
            description: "test agent".into(),
            repository: None,
            website: None,
            authors: None,
            license: None,
            icon: None,
            distribution,
        }
    }

    fn npx_dist(
        package: &str,
        args: Option<Vec<String>>,
    ) -> HashMap<Distribution, DistributionCommand> {
        let mut dist = HashMap::new();
        dist.insert(
            Distribution::Npx,
            DistributionCommand::Package(PackageDistribution {
                package: package.into(),
                args,
                env: None,
            }),
        );
        dist
    }

    fn test_state() -> Arc<Mutex<PluginState>> {
        Arc::new(Mutex::new(PluginState::new()))
    }

    // -----------------------------------------------------------------------
    // Tests for fetch_agent_from_registry (async, preference skips I/O)
    // -----------------------------------------------------------------------

    #[test]
    fn resolve_npx_with_preference() {
        let entry = entry_with_distribution("test-agent", npx_dist("my-agent", None));
        let config = DistributionsConfig::default();
        let result = futures_lite::future::block_on(fetch_agent_from_registry(
            &entry,
            Some(Distribution::Npx),
            &config,
        ));
        let assistant = result.unwrap();
        assert!(
            matches!(&assistant, Assistant::CustomStdio { name, command, args }
                if name == "test-agent" && command == "npx" && args == &vec!["my-agent"])
        );
    }

    #[test]
    fn resolve_npx_includes_dist_args() {
        let entry = entry_with_distribution(
            "test-agent",
            npx_dist(
                "my-agent",
                Some(vec!["--verbose".into(), "--port".into(), "8080".into()]),
            ),
        );
        let config = DistributionsConfig::default();
        let result = futures_lite::future::block_on(fetch_agent_from_registry(
            &entry,
            Some(Distribution::Npx),
            &config,
        ));
        let assistant = result.unwrap();
        assert!(
            matches!(&assistant, Assistant::CustomStdio { name, command, args }
                if name == "test-agent"
                   && command == "npx"
                   && args == &vec!["my-agent", "--verbose", "--port", "8080"])
        );
    }

    #[test]
    fn resolve_nonexistent_preference_returns_error() {
        let entry = entry_with_distribution("test-agent", HashMap::new());
        let config = DistributionsConfig::default();
        let result = futures_lite::future::block_on(fetch_agent_from_registry(
            &entry,
            Some(Distribution::Npx),
            &config,
        ));
        assert!(result.is_err());
    }

    #[test]
    fn resolve_disabled_distribution_returns_error() {
        let entry = entry_with_distribution("test-agent", npx_dist("my-agent", None));
        let config = DistributionsConfig {
            npx: false,
            ..Default::default()
        };
        let result = futures_lite::future::block_on(fetch_agent_from_registry(
            &entry,
            Some(Distribution::Npx),
            &config,
        ));
        assert!(result.is_err());
        assert!(
            result.unwrap_err().to_string().contains("disabled"),
            "Error should mention distribution is disabled"
        );
    }

    #[test]
    fn resolve_all_disabled_error_mentions_toggle() {
        let entry = entry_with_distribution("test-agent", npx_dist("my-agent", None));
        let config = DistributionsConfig {
            npx: false,
            uvx: false,
            binary: crate::nvim::configuration::BinaryConfig {
                enabled: false,
                ..Default::default()
            },
        };
        let result =
            futures_lite::future::block_on(fetch_agent_from_registry(&entry, None, &config));
        let err = result.unwrap_err().to_string();
        assert!(
            err.contains("disabled"),
            "Error should mention distributions are disabled"
        );
    }

    #[test]
    fn resolve_no_supported_distribution_error_mentions_agent_id() {
        let entry = entry_with_distribution("my-agent", HashMap::new());
        let config = DistributionsConfig::default();
        let result =
            futures_lite::future::block_on(fetch_agent_from_registry(&entry, None, &config));
        let err = result.unwrap_err().to_string();
        assert!(
            err.contains("no supported distribution"),
            "Error should describe no supported distribution"
        );
    }

    #[test]
    fn resolve_binary_no_platform_match_returns_error() {
        let mut dist = HashMap::new();
        dist.insert(
            Distribution::Binary,
            DistributionCommand::BinaryTargets(HashMap::new()),
        );
        let entry = entry_with_distribution("test-agent", dist);
        let config = DistributionsConfig::default();
        let result = futures_lite::future::block_on(fetch_agent_from_registry(
            &entry,
            Some(Distribution::Binary),
            &config,
        ));
        assert!(result.is_err());
    }
}
