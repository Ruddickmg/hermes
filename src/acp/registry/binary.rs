use std::collections::HashMap;
use std::path::{Path, PathBuf};

use crate::acp::Result;
use crate::acp::error::Error;
use crate::acp::registry::BinaryPlatformTarget;

/// Map the current platform (`std::env::consts::{ARCH, OS}`) to a registry
/// platform key (e.g. `"x86_64-linux"`) and look up the corresponding
/// [`BinaryPlatformTarget`].
///
/// Returns `None` when no target matches the current platform.
pub fn platform_target(
    targets: &HashMap<String, BinaryPlatformTarget>,
) -> Option<&BinaryPlatformTarget> {
    let key = format!("{}-{}", std::env::consts::ARCH, current_os());
    targets.get(&key)
}

/// Return the cached binary path for *agent_id*`/`*version*, downloading and
/// extracting the archive on cache miss.
///
/// Cache layout (XDG data home):
/// ```text
/// $XDG_DATA_HOME/hermes/agents/<agent-id>/<version>/<cmd>
/// ```
///
/// Before extracting a fresh download the entire `<agent-id>/` directory is
/// removed so that stale versions do not accumulate.
pub async fn get_binary(
    agent_id: &str,
    version: &str,
    target: &BinaryPlatformTarget,
) -> Result<PathBuf> {
    let cache_dir = cache_dir_for(agent_id, version);
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
    let cache_dir = cache_dir;
    let cmd = target.cmd.clone();
    let agent_id = agent_id.to_owned();

    blocking::unblock(move || -> Result<PathBuf> {
        std::fs::create_dir_all(&cache_dir)
            .map_err(|e| Error::Network(format!("Failed to create cache dir: {e}")))?;

        let archive_path = cache_dir.join("archive.tmp");

        // Download archive.
        let mut response = ureq::get(&url)
            .call()
            .map_err(|e| Error::Network(format!("Failed to download {url}: {e}")))?;
        let data = response
            .body_mut()
            .read_to_vec()
            .map_err(|e| Error::Network(format!("Failed to read response body: {e}")))?;
        std::fs::write(&archive_path, &data)
            .map_err(|e| Error::Network(format!("Failed to write archive to disk: {e}")))?;

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
        Ok(binary_path)
    })
    .await
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

fn cache_dir_for(agent_id: &str, version: &str) -> PathBuf {
    let base = if cfg!(target_os = "windows") {
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
    };
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
    archive
        .unpack(dest)
        .map_err(|e| Error::Network(format!("Failed to extract tar archive: {e}")))
}

fn extract_zip(archive_path: &Path, dest: &Path) -> Result<()> {
    let file = std::fs::File::open(archive_path).map_err(|e| Error::Network(format!("{e}")))?;
    let mut archive = zip::ZipArchive::new(file)
        .map_err(|e| Error::Network(format!("Failed to read zip: {e}")))?;
    archive
        .extract(dest)
        .map_err(|e| Error::Network(format!("Failed to extract zip archive: {e}")))
}

#[cfg(unix)]
fn make_executable(path: &Path) -> Result<()> {
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o755))
        .map_err(|e| Error::Network(format!("Failed to make binary executable: {e}")))
}

#[cfg(not(unix))]
fn make_executable(_path: &Path) -> Result<()> {
    Ok(())
}

#[cfg(unix)]
fn ensure_executable(path: &Path) -> Result<()> {
    use std::os::unix::fs::PermissionsExt;
    let meta = std::fs::metadata(path)
        .map_err(|e| Error::Network(format!("Failed to stat binary: {e}")))?;
    if meta.permissions().mode() & 0o111 == 0 {
        make_executable(path)?;
    }
    Ok(())
}

#[cfg(not(unix))]
fn ensure_executable(_path: &Path) -> Result<()> {
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use tempfile::TempDir;

    #[test]
    fn current_os_maps_macos_to_darwin() {
        // Not testing the actual env (depends on build host); just verify the
        // mapping function behaves correctly.
        assert_eq!(
            super::current_os(),
            std::env::consts::OS.replace("macos", "darwin")
        );
    }

    #[test]
    fn cache_dir_ends_with_expected_suffix() {
        // The base directory depends on env vars that may be shared across
        // parallel tests, so only verify the suffix (agent-id / version).
        let dir = cache_dir_for("my-agent", "1.2.3");
        assert!(
            dir.ends_with("hermes/agents/my-agent/1.2.3")
                || dir.ends_with("hermes\\agents\\my-agent\\1.2.3")
        );
    }

    #[test]
    fn cache_dir_includes_cmd_name() {
        let dir = cache_dir_for("agent-x", "0.0.1");
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
    fn platform_key_format() {
        // Verify the platform key is built as expected.
        let mut targets = HashMap::new();
        targets.insert(
            format!("{}-{}", std::env::consts::ARCH, super::current_os()),
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
}
