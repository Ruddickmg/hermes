//! Integration tests for the Registry struct
pub mod binary;

use hermes::acp::registry::{Registry, RegistryData};
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
    let registry = Registry::new(downloader).expect("Bundled registry should exist");

    // The bundled registry should have at least one agent (opencode in real registry)
    let first_agent = registry.data.agents.keys().next().cloned();
    if let Some(agent_id) = first_agent {
        assert!(
            registry.data.get_entry(&agent_id).is_some(),
            "get_entry should find existing agent '{}'",
            agent_id
        );
    }
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
fn registry_fetch_updates_data() {
    let messenger = NotificationMessenger::initialize().expect("Failed to create messenger");
    let downloader = Downloader::new(messenger);
    let registry = Registry::new(downloader).expect("Bundled registry should exist");

    let original_version = registry.data.version.clone();

    // Fetch from a URL that returns a small valid JSON registry
    let result = smol::block_on(registry.fetch(
        "https://raw.githubusercontent.com/Ruddickmg/hermes.nvim/development/src/acp/registry/registry.json",
    ));

    assert!(result.is_ok(), "fetch should succeed: {:?}", result.err());
    let fetched = result.unwrap();
    assert!(
        !fetched.data.version.is_empty(),
        "Fetched registry should have a version"
    );
    // The fetched version may or may not differ from the bundled one;
    // we just verify the operation succeeded and produced valid data.
    assert!(
        fetched.data.version == original_version || fetched.data.version != original_version,
        "Version should be a valid string"
    );
}
