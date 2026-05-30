use std::collections::HashMap;

use crate::acp::{
    Result,
    connection::Assistant,
    error::Error,
    registry::{BinaryPlatformTarget, DistributionConfig, PackageDistribution, binary},
    utilities::command::command_available,
};

pub enum DistributionSelection<'a> {
    Npx(&'a PackageDistribution),
    Uvx(&'a PackageDistribution),
    Binary(&'a BinaryPlatformTarget),
}

pub fn select_distribution<'a>(
    distribution: &'a HashMap<String, DistributionConfig>,
    preference: Option<&str>,
) -> Option<DistributionSelection<'a>> {
    let candidates: Vec<&str> = if let Some(pref) = preference {
        vec![pref]
    } else {
        vec!["npx", "uvx", "binary"]
    };

    for dist_type in &candidates {
        match (*dist_type, distribution.get(*dist_type)) {
            ("npx", Some(DistributionConfig::Package(pkg))) => {
                return Some(DistributionSelection::Npx(pkg));
            }
            ("uvx", Some(DistributionConfig::Package(pkg))) => {
                return Some(DistributionSelection::Uvx(pkg));
            }
            ("binary", Some(DistributionConfig::BinaryTargets(targets))) => {
                if let Some(target) = binary::platform_target(targets) {
                    return Some(DistributionSelection::Binary(target));
                }
            }
            _ => {}
        }
    }

    None
}

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

pub async fn resolve_agent_from_registry(
    agent_id: &str,
    entry: &crate::acp::registry::AgentEntry,
    preference: Option<&str>,
) -> Result<Assistant> {
    let candidates: Vec<&str> = if let Some(pref) = preference {
        vec![pref]
    } else {
        vec!["npx", "uvx", "binary"]
    };

    for dist_type in &candidates {
        match (*dist_type, entry.distribution.get(*dist_type)) {
            ("npx", Some(DistributionConfig::Package(pkg))) => {
                if preference.is_some() || command_available("npx").await {
                    return Ok(make_package_assistant(agent_id, "npx", pkg));
                }
            }
            ("uvx", Some(DistributionConfig::Package(pkg))) => {
                if preference.is_some() || command_available("uvx").await {
                    return Ok(make_package_assistant(agent_id, "uvx", pkg));
                }
            }
            ("binary", Some(DistributionConfig::BinaryTargets(targets))) => {
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

    if let Some(pref) = preference {
        Err(Error::InvalidInput(format!(
            "Agent '{agent_id}' has no '{pref}' distribution"
        )))
    } else {
        Err(Error::InvalidInput(format!(
            "Agent '{agent_id}' has no supported distribution for this platform"
        )))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry_with_distribution(
        id: &str,
        distribution: HashMap<String, DistributionConfig>,
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
    // Tests for select_distribution (pure, no I/O)
    // -----------------------------------------------------------------------

    #[test]
    fn select_npx_with_preference() {
        let dist = npx_dist("my-agent", None);
        let selection = select_distribution(&dist, Some("npx"));
        assert!(matches!(selection, Some(DistributionSelection::Npx(_))));
    }

    #[test]
    fn select_uvx_with_preference() {
        let dist = uvx_dist("uvx-agent");
        let selection = select_distribution(&dist, Some("uvx"));
        assert!(matches!(selection, Some(DistributionSelection::Uvx(_))));
    }

    #[test]
    fn select_binary_with_platform_match() {
        let os = std::env::consts::OS.replace("macos", "darwin");
        let mut targets = HashMap::new();
        targets.insert(
            format!("{}-{}", std::env::consts::ARCH, os),
            BinaryPlatformTarget {
                archive: "https://example.com/agent.tar.gz".into(),
                cmd: "agent".into(),
                args: None,
                env: None,
            },
        );
        let mut dist = HashMap::new();
        dist.insert("binary".into(), DistributionConfig::BinaryTargets(targets));
        let selection = select_distribution(&dist, Some("binary"));
        assert!(matches!(selection, Some(DistributionSelection::Binary(_))));
    }

    #[test]
    fn select_nonexistent_preference_returns_none() {
        let dist = npx_dist("my-agent", None);
        let selection = select_distribution(&dist, Some("bad-dist"));
        assert!(selection.is_none());
    }

    #[test]
    fn select_empty_distribution_returns_none() {
        let dist = HashMap::new();
        let selection = select_distribution(&dist, Some("npx"));
        assert!(selection.is_none());
    }

    #[test]
    fn select_auto_prioritizes_npx() {
        let mut dist = HashMap::new();
        dist.insert(
            "npx".into(),
            DistributionConfig::Package(PackageDistribution {
                package: "agent".into(),
                args: None,
                env: None,
            }),
        );
        dist.insert(
            "uvx".into(),
            DistributionConfig::Package(PackageDistribution {
                package: "agent".into(),
                args: None,
                env: None,
            }),
        );
        let selection = select_distribution(&dist, None);
        assert!(matches!(selection, Some(DistributionSelection::Npx(_))));
    }

    #[test]
    fn select_binary_no_platform_match_returns_none() {
        let mut dist = HashMap::new();
        dist.insert(
            "binary".into(),
            DistributionConfig::BinaryTargets(HashMap::new()),
        );
        let selection = select_distribution(&dist, Some("binary"));
        assert!(selection.is_none());
    }

    #[test]
    fn select_auto_falls_back_to_uvx_when_no_npx() {
        let mut dist = HashMap::new();
        dist.insert(
            "uvx".into(),
            DistributionConfig::Package(PackageDistribution {
                package: "uvx-agent".into(),
                args: None,
                env: None,
            }),
        );
        let selection = select_distribution(&dist, None);
        assert!(matches!(selection, Some(DistributionSelection::Uvx(_))));
    }

    #[test]
    fn select_auto_falls_back_to_binary_when_no_package() {
        let os = std::env::consts::OS.replace("macos", "darwin");
        let mut targets = HashMap::new();
        targets.insert(
            format!("{}-{}", std::env::consts::ARCH, os),
            BinaryPlatformTarget {
                archive: "https://example.com/agent.tar.gz".into(),
                cmd: "agent".into(),
                args: None,
                env: None,
            },
        );
        let mut dist = HashMap::new();
        dist.insert("binary".into(), DistributionConfig::BinaryTargets(targets));
        let selection = select_distribution(&dist, None);
        assert!(matches!(selection, Some(DistributionSelection::Binary(_))));
    }

    #[test]
    fn select_auto_returns_none_when_no_distributions() {
        let dist = HashMap::new();
        let selection = select_distribution(&dist, None);
        assert!(selection.is_none());
    }

    #[test]
    fn select_preference_npx_on_uvx_only_returns_none() {
        let dist = uvx_dist("uvx-agent");
        let selection = select_distribution(&dist, Some("npx"));
        assert!(selection.is_none());
    }

    #[test]
    fn select_preference_uvx_on_npx_only_returns_none() {
        let dist = npx_dist("npx-agent", None);
        let selection = select_distribution(&dist, Some("uvx"));
        assert!(selection.is_none());
    }

    // -----------------------------------------------------------------------
    // Tests for resolve_agent_from_registry (async, preference skips I/O)
    // -----------------------------------------------------------------------

    #[test]
    fn resolve_npx_with_preference() {
        let entry = entry_with_distribution("test-agent", npx_dist("my-agent", None));
        let result = futures_lite::future::block_on(resolve_agent_from_registry(
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
        let result = futures_lite::future::block_on(resolve_agent_from_registry(
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
        let result = futures_lite::future::block_on(resolve_agent_from_registry(
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
        let result = futures_lite::future::block_on(resolve_agent_from_registry(
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
        let result = futures_lite::future::block_on(resolve_agent_from_registry(
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
        let result = futures_lite::future::block_on(resolve_agent_from_registry(
            "test-agent",
            &entry,
            Some("binary"),
        ));
        assert!(result.is_err());
    }

    #[test]
    fn resolve_nonexistent_preference_error_mentions_agent_id() {
        let entry = entry_with_distribution("my-agent", npx_dist("my-agent", None));
        let result = futures_lite::future::block_on(resolve_agent_from_registry(
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
            futures_lite::future::block_on(resolve_agent_from_registry("my-agent", &entry, None));
        let err = result.unwrap_err().to_string();
        assert!(err.contains("my-agent"), "Error should mention agent id");
        assert!(
            err.contains("no supported distribution"),
            "Error should describe no supported distribution"
        );
    }
}
