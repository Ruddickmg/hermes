use std::collections::HashSet;
use std::fs;
use std::io::{self, Write};
use std::path::PathBuf;

use super::LogSink;

const HISTORY_FLUSH_INTERVAL: usize = 20;

/// A sink that writes JSONL history entries to per-session files.
///
/// Uses `write_keyed(path, message)` to route entries. Paths are
/// relative to `base_path` (e.g., `"opencode/session-abc.jsonl"`).
/// Parent directories are created on first write to each path.
pub struct HistorySink {
    base_path: PathBuf,
    created_dirs: HashSet<PathBuf>,
    buffer: Vec<(String, String)>,
    flush_interval: usize,
}

impl HistorySink {
    pub fn new(base_path: PathBuf) -> io::Result<Self> {
        fs::create_dir_all(&base_path)?;
        Ok(Self {
            base_path,
            created_dirs: HashSet::new(),
            buffer: Vec::new(),
            flush_interval: HISTORY_FLUSH_INTERVAL,
        })
    }

    fn flush_buffer(&mut self) -> io::Result<()> {
        if self.buffer.is_empty() {
            return Ok(());
        }

        // Group entries by path
        let mut entries: Vec<(String, Vec<String>)> = Vec::new();
        for (path, msg) in self.buffer.drain(..) {
            if let Some((_, msgs)) = entries.iter_mut().find(|(p, _)| *p == path) {
                msgs.push(msg);
            } else {
                entries.push((path, vec![msg]));
            }
        }

        for (key, msgs) in entries {
            let path = self.base_path.join(&key);
            if let Some(parent) = path.parent() {
                if self.created_dirs.insert(parent.to_path_buf()) {
                    fs::create_dir_all(parent)?;
                }
            }
            let mut file = fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(&path)?;
            for msg in &msgs {
                writeln!(file, "{}", msg)?;
            }
        }
        Ok(())
    }
}

impl LogSink for HistorySink {
    fn write_batch(&mut self, _messages: &[String]) -> io::Result<()> {
        Ok(())
    }

    fn write_keyed(&mut self, path: &str, message: &str) -> io::Result<()> {
        self.buffer.push((path.to_string(), message.to_string()));

        if self.buffer.len() >= self.flush_interval {
            self.flush_buffer()?;
        }
        Ok(())
    }

    fn flush(&mut self) -> io::Result<()> {
        self.flush_buffer()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;
    use tempfile::TempDir;

    fn create_sink(temp_dir: &TempDir) -> HistorySink {
        let path = temp_dir.path().join("history");
        HistorySink::new(path).unwrap()
    }

    #[test]
    fn test_history_sink_writes_per_path() {
        let temp_dir = TempDir::new().unwrap();
        let mut sink = create_sink(&temp_dir);

        sink.write_keyed(
            "opencode/session-1.jsonl",
            r#"{"role":"user","content":"hello"}"#,
        )
        .unwrap();
        sink.write_keyed(
            "opencode/session-1.jsonl",
            r#"{"role":"assistant","content":"hi"}"#,
        )
        .unwrap();
        sink.flush().unwrap();

        let contents =
            fs::read_to_string(temp_dir.path().join("history/opencode/session-1.jsonl")).unwrap();
        let lines: Vec<&str> = contents.lines().collect();
        assert_eq!(lines.len(), 2);
    }

    #[test]
    fn test_history_sink_separates_paths() {
        let temp_dir = TempDir::new().unwrap();
        let mut sink = create_sink(&temp_dir);

        sink.write_keyed("open-code/session-a.jsonl", "line-a1")
            .unwrap();
        sink.write_keyed("copilot/session-b.jsonl", "line-b1")
            .unwrap();
        sink.write_keyed("open-code/session-a.jsonl", "line-a2")
            .unwrap();
        sink.flush().unwrap();

        let contents_a =
            fs::read_to_string(temp_dir.path().join("history/open-code/session-a.jsonl")).unwrap();
        let contents_b =
            fs::read_to_string(temp_dir.path().join("history/copilot/session-b.jsonl")).unwrap();

        assert_eq!(
            contents_a.lines().collect::<Vec<_>>(),
            ["line-a1", "line-a2"]
        );
        assert_eq!(contents_b.lines().collect::<Vec<_>>(), ["line-b1"]);
    }

    #[test]
    fn test_history_sink_flushes_on_interval() {
        let temp_dir = TempDir::new().unwrap();
        let mut sink = create_sink(&temp_dir);

        for i in 0..25 {
            sink.write_keyed("agent/session-1.jsonl", &format!("msg-{}", i))
                .unwrap();
        }
        sink.flush().unwrap();

        let contents =
            fs::read_to_string(temp_dir.path().join("history/agent/session-1.jsonl")).unwrap();
        let lines: Vec<&str> = contents.lines().collect();
        assert_eq!(lines.len(), 25);
    }

    #[test]
    fn test_history_sink_appends_on_multiple_flushes() {
        let temp_dir = TempDir::new().unwrap();
        let mut sink = create_sink(&temp_dir);

        sink.write_keyed("agent/session-1.jsonl", "first").unwrap();
        sink.flush().unwrap();
        sink.write_keyed("agent/session-1.jsonl", "second").unwrap();
        sink.flush().unwrap();

        let contents =
            fs::read_to_string(temp_dir.path().join("history/agent/session-1.jsonl")).unwrap();
        let lines: Vec<&str> = contents.lines().collect();
        assert_eq!(lines, ["first", "second"]);
    }

    #[test]
    fn test_history_sink_creates_agent_directory_on_first_write() {
        let temp_dir = TempDir::new().unwrap();
        let mut sink = create_sink(&temp_dir);

        let agent_dir = temp_dir.path().join("history/my-agent");
        assert!(!agent_dir.exists());

        sink.write_keyed("my-agent/session-1.jsonl", "hello")
            .unwrap();
        sink.flush().unwrap();

        assert!(
            agent_dir.exists(),
            "Agent subdirectory should be created on first write"
        );
    }
}
