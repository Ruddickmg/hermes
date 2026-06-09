//! Integration tests for the Registry struct
pub mod binary;

use hermes::acp::registry::{Registry, RegistryData, entry::AgentEntry};
use hermes::utilities::{Downloader, NotificationMessenger};
use std::collections::HashMap;

#[nvim_oxi::test]
fn registry_new_returns_some_with_bundled_data() {
    let messenger = NotificationMessenger::initialize().expect("Failed to create messenger");
    let downloader = Downloader::new(messenger);
    let registry = Registry::new(downloader);
    assert!(
        registry.is_some(),
        "Registry::new should return Some when bundled data exists"
    );
}

#[nvim_oxi::test]
fn registry_data_get_entry_finds_existing_agent() {
    let messenger = NotificationMessenger::initialize().expect("Failed to create messenger");
    let downloader = Downloader::new(messenger);
    let mut registry = Registry::new(downloader).expect("Bundled registry should exist");
    // Inject a known agent to avoid depending on bundled data contents.
    registry.data.agents.insert(
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

    assert!(
        registry.data.get_entry("test-agent").is_some(),
        "get_entry should find existing agent"
    );
}

#[nvim_oxi::test]
fn registry_can_be_inserted_into_hash_set() {
    let messenger = NotificationMessenger::initialize().expect("Failed to create messenger");
    let downloader = Downloader::new(messenger);
    let registry = Registry::new(downloader).expect("Bundled registry should exist");

    let mut set = std::collections::HashSet::new();
    set.insert(registry.clone());
    assert!(
        set.contains(&registry),
        "Registry should be usable as a HashSet key"
    );
}

#[nvim_oxi::test]
fn registry_fetch_succeeds() {
    let messenger = NotificationMessenger::initialize().expect("Failed to create messenger");
    let downloader = Downloader::new(messenger);
    let registry = Registry::new(downloader).expect("Bundled registry should exist");

    let result = smol::block_on(registry.fetch(
        "https://raw.githubusercontent.com/Ruddickmg/hermes.nvim/development/src/acp/registry/registry.json",
    ));

    assert!(result.is_ok(), "fetch should succeed: {:?}", result.err());
}

#[nvim_oxi::test]
fn registry_fetch_returns_data_with_version() {
    let messenger = NotificationMessenger::initialize().expect("Failed to create messenger");
    let downloader = Downloader::new(messenger);
    let registry = Registry::new(downloader).expect("Bundled registry should exist");

    let fetched = smol::block_on(registry.fetch(
        "https://raw.githubusercontent.com/Ruddickmg/hermes.nvim/development/src/acp/registry/registry.json",
    ))
    .expect("fetch should succeed");

    assert!(
        !fetched.data.version.is_empty(),
        "Fetched registry should have a version"
    );
}
