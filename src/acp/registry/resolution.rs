use crate::acp::{
    Result,
    connection::Assistant,
    error::Error,
    registry::{
        BinaryPlatformTarget, DistributionConfig, PackageDistribution, binary,
        distribution::Distribution,
    },
    utilities::command::command_available,
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
) -> Result<Assistant> {
    let agent_id = &entry.id;
    let candidates: Vec<Distribution> = preference
        .map(|p| vec![p])
        .unwrap_or_else(|| entry.distributions())
        .into_iter()
        .filter(|distribution| entry.has_distribution(distribution))
        .collect();

    for dist_type in &candidates {
        match entry.get_distribution(dist_type) {
            Some(DistributionConfig::Package(pkg)) => {
                if command_available(&dist_type.to_string()).await {
                    return Ok(make_package_assistant(
                        agent_id,
                        &dist_type.to_string(),
                        pkg,
                    ));
                }
            }
            Some(DistributionConfig::BinaryTargets(targets)) => {
                if let Some(target) = binary::platform_target(targets) {
                    let path = binary::get_binary(agent_id, &entry.version, target).await?;
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

    Err(Error::InvalidInput(format!(
        "Agent '{}' has no supported distribution for this platform",
        entry.id
    )))
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::*;

    fn entry_with_distribution(
        id: &str,
        distribution: HashMap<Distribution, DistributionConfig>,
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

    fn npx_dist(package: &str, args: Option<Vec<String>>) -> HashMap<String, DistributionConfig> {
        let mut dist = HashMap::new();
        dist.insert(
            "npx".into(),
            DistributionConfig::Package(PackageDistribution {
                package: package.into(),
                args,
                env: None,
            }),
        );
        dist
    }

    fn uvx_dist(package: &str) -> HashMap<String, DistributionConfig> {
        let mut dist = HashMap::new();
        dist.insert(
            "uvx".into(),
            DistributionConfig::Package(PackageDistribution {
                package: package.into(),
                args: None,
                env: None,
            }),
        );
        dist
    }

    // -----------------------------------------------------------------------
    // Tests for resolve_agent_from_registry (async, preference skips I/O)
    // -----------------------------------------------------------------------

    #[test]
    fn resolve_npx_with_preference() {
        let entry = entry_with_distribution("test-agent", npx_dist("my-agent", None));
        let result = futures_lite::future::block_on(fetch_agent_from_registry(
            "test-agent",
            &entry,
            Some("npx"),
        ));
        let assistant = result.unwrap();
        assert!(
            matches!(&assistant, Assistant::CustomStdio { name, command, args }
                if name == "test-agent" && command == "npx" && args == &vec!["my-agent"])
        );
    }

    #[test]
    fn resolve_uvx_with_preference() {
        let entry = entry_with_distribution("test-agent", uvx_dist("uvx-agent"));
        let result = futures_lite::future::block_on(fetch_agent_from_registry(
            "test-agent",
            &entry,
            Some("uvx"),
        ));
        let assistant = result.unwrap();
        assert!(
            matches!(&assistant, Assistant::CustomStdio { name, command, args }
                if name == "test-agent" && command == "uvx" && args == &vec!["uvx-agent"])
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
        let result = futures_lite::future::block_on(fetch_agent_from_registry(
            "test-agent",
            &entry,
            Some("npx"),
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
    fn resolve_nonexistent_preference_errors() {
        let entry = entry_with_distribution("test-agent", npx_dist("my-agent", None));
        let result = futures_lite::future::block_on(fetch_agent_from_registry(
            "test-agent",
            &entry,
            Some("bad-dist"),
        ));
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("bad-dist"));
    }

    #[test]
    fn resolve_empty_distribution_errors() {
        let entry = entry_with_distribution("test-agent", HashMap::new());
        let result = futures_lite::future::block_on(fetch_agent_from_registry(
            "test-agent",
            &entry,
            Some("npx"),
        ));
        assert!(result.is_err());
    }

    #[test]
    fn resolve_binary_no_platform_match_errors() {
        let mut dist = HashMap::new();
        dist.insert(
            "binary".into(),
            DistributionConfig::BinaryTargets(HashMap::new()),
        );
        let entry = entry_with_distribution("test-agent", dist);
        let result = futures_lite::future::block_on(fetch_agent_from_registry(
            "test-agent",
            &entry,
            Some("binary"),
        ));
        assert!(result.is_err());
    }

    #[test]
    fn resolve_nonexistent_preference_error_mentions_agent_id() {
        let entry = entry_with_distribution("my-agent", npx_dist("my-agent", None));
        let result = futures_lite::future::block_on(fetch_agent_from_registry(
            "my-agent",
            &entry,
            Some("bad-dist"),
        ));
        let err = result.unwrap_err().to_string();
        assert!(err.contains("my-agent"), "Error should mention agent id");
    }

    #[test]
    fn resolve_no_supported_distribution_error_mentions_agent_id() {
        let entry = entry_with_distribution("my-agent", HashMap::new());
        let result =
            futures_lite::future::block_on(fetch_agent_from_registry("my-agent", &entry, None));
        let err = result.unwrap_err().to_string();
        assert!(err.contains("my-agent"), "Error should mention agent id");
        assert!(
            err.contains("no supported distribution"),
            "Error should describe no supported distribution"
        );
    }
}
