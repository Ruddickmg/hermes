//! Integration tests for Assistant command construction

use hermes::acp::{connection::Assistant, registry::entry::AgentEntry};
use hermes::nvim::configuration::DistributionsConfig;
use std::collections::HashMap;

#[nvim_oxi::test]
fn assistant_command_with_no_registry_returns_error() {
    let assistant = Assistant::Registered {
        agent: AgentEntry {
            id: "test-agent".to_string(),
            name: "Test Agent".to_string(),
            version: "1.0.0".to_string(),
            description: "test".to_string(),
            repository: None,
            website: None,
            authors: None,
            license: None,
            icon: None,
            distribution: HashMap::new(),
        },
        distribution: None,
        configuration: DistributionsConfig::default(),
        command: None,
        args: None,
        registry: None,
    };

    let result = smol::block_on(assistant.command());
    assert!(
        result.is_err(),
        "command() should fail when registry is None"
    );
}

#[nvim_oxi::test]
fn assistant_command_with_no_registry_error_mentions_registry() {
    let assistant = Assistant::Registered {
        agent: AgentEntry {
            id: "test-agent".to_string(),
            name: "Test Agent".to_string(),
            version: "1.0.0".to_string(),
            description: "test".to_string(),
            repository: None,
            website: None,
            authors: None,
            license: None,
            icon: None,
            distribution: HashMap::new(),
        },
        distribution: None,
        configuration: DistributionsConfig::default(),
        command: None,
        args: None,
        registry: None,
    };

    let result = smol::block_on(assistant.command());
    let err = result.unwrap_err().to_string();
    assert!(
        err.contains("registry") || err.contains("Registry"),
        "Error should mention missing registry: {}",
        err
    );
}
