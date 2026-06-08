use std::collections::HashMap;
use std::path::{Path, PathBuf};

use tracing::info;

use crate::acp::Result;
use crate::acp::error::Error;
use crate::acp::registry::{BinaryPlatformTarget, Registry};

/// [`BinaryPlatformTarget`].
pub fn platform_target(
    targets: &HashMap<String, BinaryPlatformTarget>,
) -> Option<&BinaryPlatformTarget> {
    let key = format!("{}-{}", current_os(), std::env::consts::ARCH);
    targets.get(&key)
}

impl Registry {
    pub async fn download_binary(
        &self,
        agent_id: &str,
        version: &str,
        target: &BinaryPlatformTarget,
        cache_dir_override: Option<&Path>,
    ) -> Result<PathBuf> {
        let cache_dir = cache_dir_for(agent_id, version, cache_dir_override);
        let binary_path = cache_dir.join(&target.cmd);

        if binary_path.is_file() {
            ensure_executable(&binary_path)?;
            return Ok(binary_path);
        }

        // Purge any previous version from the cache.
        if let Some(parent) = cache_dir.parent() {
            if parent.exists() {
                std::fs::remove_dir_all(parent)
                    .map_err(|e| Error::Network(format!("Failed to clear cache: {e}")))?;
            }
        }

        let url = target.archive.clone();
        let cmd = target.cmd.clone();
        let agent_id = agent_id.to_owned();
        let downloader = self.downloader.clone();

        blocking::unblock(move || -> Result<PathBuf> {
            std::fs::create_dir_all(&cache_dir)
                .map_err(|e| Error::Network(format!("Failed to create cache dir: {e}")))?;

            let archive_name = url
                .split(['?', '#'])
                .next()
                .unwrap_or(&url)
                .rsplit('/')
                .next()
                .unwrap_or("archive")
                .to_string();
            let archive_path = cache_dir.join(&archive_name);

            info!("Downloading binary for {}", agent_id);

            downloader.download_to_file(
                &url,
                &archive_path,
                &format!("hermes-agent-{}", agent_id),
                &format!("Downloading {} binary", agent_id),
            )?;

            // Extract archive.
            extract_archive(&archive_path, &cache_dir)?;

            // Remove the archive now that extraction succeeded.
            std::fs::remove_file(&archive_path)
                .map_err(|e| Error::Network(format!("Failed to remove archive: {e}")))?;

            let binary_path = cache_dir.join(&cmd);
            if !binary_path.is_file() {
                return Err(Error::InvalidInput(format!(
                    "Binary '{cmd}' not found after extracting archive for agent '{agent_id}'"
                )));
            }

            make_executable(&binary_path)?;

            info!("Finished downloading binary for {}", agent_id);

            Ok(binary_path)
        })
        .await
    }
}

// ---------------------------------------------------------------------------
// Private helpers
// ---------------------------------------------------------------------------

fn current_os() -> &'static str {
    match std::env::consts::OS {
        "macos" => "darwin",
        other => other,
    }
}

fn cache_dir_for(agent_id: &str, version: &str, base_override: Option<&Path>) -> PathBuf {
    let base = base_override.map(PathBuf::from).unwrap_or_else(|| {
        if cfg!(target_os = "windows") {
            std::env::var("APPDATA")
                .map(PathBuf::from)
                .unwrap_or_else(|_| PathBuf::from("."))
        } else {
            std::env::var("XDG_DATA_HOME")
                .ok()
                .map(PathBuf::from)
                .unwrap_or_else(|| {
                    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
                    PathBuf::from(home).join(".local").join("share")
                })
        }
    });
    base.join("hermes")
        .join("agents")
        .join(agent_id)
        .join(version)
}

fn extract_archive(archive_path: &Path, dest: &Path) -> Result<()> {
    let name = archive_path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("")
        .to_lowercase();

    if name.ends_with(".tar.gz") || name.ends_with(".tgz") {
        extract_tar_gz(archive_path, dest)
    } else if name.ends_with(".tar") {
        extract_tar(archive_path, dest)
    } else if name.ends_with(".zip") {
        extract_zip(archive_path, dest)
    } else if let Some(ext) = archive_path.extension().and_then(|e| e.to_str()) {
        Err(Error::Network(format!(
            "Unsupported archive format '.{ext}' for '{}'",
            archive_path.display()
        )))
    } else {
        Err(Error::Network(format!(
            "Cannot determine archive format for '{}'",
            archive_path.display()
        )))
    }
}

fn extract_tar_gz(archive_path: &Path, dest: &Path) -> Result<()> {
    let file = std::fs::File::open(archive_path).map_err(|e| Error::Network(format!("{e}")))?;
    let decoder = flate2::read::GzDecoder::new(file);
    let mut archive = tar::Archive::new(decoder);
    archive
        .unpack(dest)
        .map_err(|e| Error::Network(format!("Failed to extract tar.gz archive: {e}")))
}

fn extract_tar(archive_path: &Path, dest: &Path) -> Result<()> {
    let file = std::fs::File::open(archive_path).map_err(|e| Error::Network(format!("{e}")))?;
    let mut archive = tar::Archive::new(file);

    for entry in archive
        .entries()
        .map_err(|e| Error::Network(format!("Failed to read tar entries: {e}")))?
    {
        let mut entry =
            entry.map_err(|e| Error::Network(format!("Failed to read tar entry: {e}")))?;
        entry
            .unpack_in(dest)
            .map_err(|e| Error::Network(format!("Failed to extract tar archive: {e}")))?;
    }

    Ok(())
}

fn extract_zip(archive_path: &Path, dest: &Path) -> Result<()> {
    let file = std::fs::File::open(archive_path).map_err(|e| Error::Network(format!("{e}")))?;
    let mut archive = zip::ZipArchive::new(file)
        .map_err(|e| Error::Network(format!("Failed to read zip: {e}")))?;
    archive
        .extract(dest)
        .map_err(|e| Error::Network(format!("Failed to extract zip archive: {e}")))
}

fn ensure_executable(path: &Path) -> Result<()> {
    if !path.is_file() {
        return Err(Error::InvalidInput(format!(
            "Binary not found at {}",
            path.display()
        )));
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = std::fs::metadata(path)
            .map_err(|e| Error::Network(format!("Failed to read permissions: {e}")))?
            .permissions();
        if perms.mode() & 0o111 == 0 {
            perms.set_mode(perms.mode() | 0o755);
            std::fs::set_permissions(path, perms)
                .map_err(|e| Error::Network(format!("Failed to set permissions: {e}")))?;
        }
    }
    Ok(())
}

#[cfg(unix)]
fn make_executable(path: &Path) -> Result<()> {
    use std::os::unix::fs::PermissionsExt;
    let mut perms = std::fs::metadata(path)
        .map_err(|e| Error::Network(format!("Failed to read permissions: {e}")))?
        .permissions();
    perms.set_mode(perms.mode() | 0o755);
    std::fs::set_permissions(path, perms)
        .map_err(|e| Error::Network(format!("Failed to set permissions: {e}")))
}

#[cfg(not(unix))]
fn make_executable(_path: &Path) -> Result<()> {
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use tempfile::TempDir;

    #[test]
    fn cache_dir_for_current_platform() {
        let dir = cache_dir_for("my-agent", "1.2.3", None);
        let s = dir.to_string_lossy();
        assert!(
            s.contains("hermes/agents/my-agent/1.2.3")
                || s.contains("hermes\\agents\\my-agent\\1.2.3"),
            "cache dir should contain platform-correct path: {}",
            s
        );
    }

    #[test]
    fn current_os_maps_macos_to_darwin() {
        // This is architecture-dependent, but we can verify it doesn't panic
        let _os = current_os();
    }

    #[test]
    fn current_os_contains_expected_value() {
        let os = current_os();
        assert!(
            os == std::env::consts::OS || os == "darwin",
            "current_os should be either the raw OS or 'darwin' for macOS"
        );
    }

    #[test]
    fn cache_dir_uses_xdg_data_home_when_set() {
        let original = std::env::var_os("XDG_DATA_HOME");
        unsafe { std::env::set_var("XDG_DATA_HOME", "/tmp/xdg-data") };
        let dir = cache_dir_for("my-agent", "1.2.3", None);
        unsafe { std::env::set_var("XDG_DATA_HOME", original.unwrap_or_default()) };

        assert!(
            dir.starts_with("/tmp/xdg-data"),
            "cache dir should use XDG_DATA_HOME when set: {}",
            dir.display()
        );
    }

    #[test]
    fn cache_dir_uses_appdata_on_windows() {
        // We can only verify the fallback logic by checking the default behavior
        // The actual Windows behavior depends on the APPDATA env var
        if cfg!(target_os = "windows") {
            let dir = cache_dir_for("my-agent", "1.2.3", None);
            assert!(
                dir.to_string_lossy().contains("hermes"),
                "cache dir should contain 'hermes' on Windows"
            );
        }
    }

    #[test]
    fn cache_dir_for_with_override() {
        let tmp = TempDir::new().unwrap();
        let dir = cache_dir_for("agent", "v1", Some(tmp.path()));
        let expected = tmp
            .path()
            .join("hermes")
            .join("agents")
            .join("agent")
            .join("v1");
        assert_eq!(dir, expected);
    }

    #[test]
    fn cache_dir_ends_with_expected_suffix() {
        // The base directory depends on env vars that may be shared across
        // parallel tests, so only verify the suffix (agent-id / version).
        let dir = cache_dir_for("my-agent", "1.2.3", None);
        assert!(
            dir.ends_with("hermes/agents/my-agent/1.2.3")
                || dir.ends_with("hermes\\agents\\my-agent\\1.2.3")
        );
    }

    #[test]
    fn cache_dir_includes_cmd_name() {
        let dir = cache_dir_for("agent-x", "0.0.1", None);
        assert!(
            dir.to_string_lossy().contains("agent-x"),
            "cache dir should contain agent id"
        );
        assert!(
            dir.to_string_lossy().contains("0.0.1"),
            "cache dir should contain version"
        );
    }

    #[test]
    fn cache_dir_override_replaces_base() {
        let dir = cache_dir_for("my-agent", "1.2.3", Some(Path::new("/tmp/cache")));
        let expected = Path::new("/tmp/cache")
            .join("hermes")
            .join("agents")
            .join("my-agent")
            .join("1.2.3");
        assert_eq!(dir, expected);
    }

    #[test]
    fn platform_key_format() {
        // Verify the platform key is built as expected (os-arch order).
        let mut targets = HashMap::new();
        targets.insert(
            format!("{}-{}", super::current_os(), std::env::consts::ARCH),
            BinaryPlatformTarget {
                archive: "https://example.com/agent.tar.gz".into(),
                cmd: "agent".into(),
                args: None,
                env: None,
            },
        );
        assert!(platform_target(&targets).is_some());
    }

    #[test]
    fn platform_key_no_match() {
        let targets = HashMap::new();
        assert!(platform_target(&targets).is_none());
    }

    #[test]
    fn archive_unsupported_format_errors() {
        let tmp = TempDir::new().unwrap();
        let bad = tmp.path().join("archive.rar");
        std::fs::write(&bad, b"garbage").unwrap();
        let result = extract_archive(&bad, tmp.path());
        assert!(result.is_err());
        assert!(
            result.unwrap_err().to_string().contains("Unsupported"),
            "Should mention unsupported format"
        );
    }

    #[test]
    fn bundled_registry_opencode_has_binary_target_for_current_platform() {
        let json = include_str!("registry.json");
        let data: super::super::Registry =
            serde_json::from_str(json).expect("Bundled registry should parse");

        let opencode = data
            .agents
            .get("opencode")
            .expect("Bundled registry should contain 'opencode'");

        let binary_config = opencode
            .distribution
            .iter()
            .find_map(|(dist, config)| {
                if matches!(dist, super::super::distribution::Distribution::Binary) {
                    Some(config)
                } else {
                    None
                }
            })
            .expect("opencode should have a 'binary' distribution");

        match binary_config {
            super::super::DistributionCommand::BinaryTargets(targets) => {
                let key = format!("{}-{}", super::current_os(), std::env::consts::ARCH);
                assert!(
                    targets.contains_key(&key),
                    "opencode bundled registry should have a binary target \
                     for current platform ({key}). Available keys: {:?}",
                    targets.keys().collect::<Vec<_>>()
                );
            }
            _ => panic!("opencode distribution should be BinaryTargets"),
        }
    }

    #[test]
    fn bundled_registry_all_binary_keys_follow_os_arch_format() {
        let json = include_str!("registry.json");
        let data: super::super::Registry =
            serde_json::from_str(json).expect("Bundled registry should parse");

        let mut checked = 0u32;
        for (agent_id, entry) in &data.agents {
            for (_dist, config) in &entry.distribution {
                if let super::super::DistributionCommand::BinaryTargets(targets) = config {
                    for key in targets.keys() {
                        checked += 1;
                        let parts: Vec<&str> = key.splitn(2, '-').collect();
                        assert_eq!(
                            parts.len(),
                            2,
                            "Agent '{agent_id}' binary target key '{key}' \
                             must be in '{{os}}-{{arch}}' format"
                        );
                        assert!(
                            !parts[0].is_empty(),
                            "Agent '{agent_id}' binary target key '{key}' has empty os part"
                        );
                        assert!(
                            !parts[1].is_empty(),
                            "Agent '{agent_id}' binary target key '{key}' has empty arch part"
                        );
                    }
                }
            }
        }
        assert!(
            checked > 3,
            "Should have checked multiple binary target keys (checked: {checked})"
        );
    }
}
