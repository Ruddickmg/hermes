//! Integration tests for registry binary operations

use hermes::acp::registry::{BinaryPlatformTarget, Registry, RegistryData};
use hermes::utilities::{Downloader, NotificationMessenger};
use std::collections::HashMap;

#[nvim_oxi::test]
fn download_binary_cache_hit_returns_existing_path() {
    let messenger = NotificationMessenger::initialize().expect("Failed to create messenger");
    let downloader = Downloader::new(messenger);
    let mut registry = Registry::new(downloader).expect("Bundled registry should exist");
    registry.data = RegistryData {
        version: "1.0.0".to_string(),
        agents: HashMap::new(),
    };

    let target = BinaryPlatformTarget {
        archive: "https://example.com/fake.tar.gz".to_string(),
        cmd: "test-binary".to_string(),
        args: None,
        env: None,
    };

    // Create a fake cache directory and binary to simulate a cache hit.
    let cache_base = std::env::temp_dir().join("hermes_test_cache");
    let agent_dir = cache_base
        .join("hermes")
        .join("agents")
        .join("test-agent")
        .join("1.0.0");
    std::fs::create_dir_all(&agent_dir).expect("Should create cache dir");
    let binary_path = agent_dir.join("test-binary");
    std::fs::write(&binary_path, b"fake binary").expect("Should write fake binary");

    // On Unix, ensure the file is executable so ensure_executable passes.
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = std::fs::metadata(&binary_path).unwrap().permissions();
        perms.set_mode(0o755);
        std::fs::set_permissions(&binary_path, perms).unwrap();
    }

    let result =
        smol::block_on(registry.download_binary("test-agent", "1.0.0", &target, Some(&cache_base)));

    assert!(
        result.is_ok(),
        "Cache hit should return existing binary path: {:?}",
        result.err()
    );
    let path = result.unwrap();
    assert_eq!(path, binary_path, "Should return the cached binary path");

    // Cleanup
    let _ = std::fs::remove_dir_all(&cache_base);
}

#[nvim_oxi::test]
#[cfg(unix)]
fn ensure_executable_adds_execute_permission() {
    let messenger = NotificationMessenger::initialize().expect("Failed to create messenger");
    let downloader = Downloader::new(messenger);
    let mut registry = Registry::new(downloader).expect("Bundled registry should exist");
    registry.data = RegistryData {
        version: "1.0.0".to_string(),
        agents: HashMap::new(),
    };

    let target = BinaryPlatformTarget {
        archive: "https://example.com/fake.tar.gz".to_string(),
        cmd: "test-binary".to_string(),
        args: None,
        env: None,
    };

    // Create a fake cache directory and binary without execute permission.
    let cache_base = std::env::temp_dir().join("hermes_test_exec");
    let agent_dir = cache_base
        .join("hermes")
        .join("agents")
        .join("exec-agent")
        .join("1.0.0");
    std::fs::create_dir_all(&agent_dir).expect("Should create cache dir");
    let binary_path = agent_dir.join("test-binary");
    std::fs::write(&binary_path, b"fake binary").expect("Should write fake binary");

    // Remove execute permission
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = std::fs::metadata(&binary_path).unwrap().permissions();
        perms.set_mode(0o644);
        std::fs::set_permissions(&binary_path, perms).unwrap();
    }

    let result =
        smol::block_on(registry.download_binary("exec-agent", "1.0.0", &target, Some(&cache_base)));

    assert!(
        result.is_ok(),
        "Should succeed after ensuring executable: {:?}",
        result.err()
    );

    // Verify permissions were updated
    {
        use std::os::unix::fs::PermissionsExt;
        let perms = std::fs::metadata(&binary_path).unwrap().permissions();
        assert!(
            perms.mode() & 0o111 != 0,
            "Binary should have execute permission"
        );
    }

    // Cleanup
    let _ = std::fs::remove_dir_all(&cache_base);
}
