//! stdio transport: spawn an agent subprocess and drive an ACP connection
//! over its stdin/stdout.
//!
//! This module owns the child-process lifecycle (via the `child` submodule)
//! and the per-protocol orchestration. The ACP `Client.builder()` plumbing is
//! shared with all other transports via
//! [`crate::acp::connection::connect::handle_connection`].

pub mod child;

use crate::{
    Handler,
    acp::{
        connection::{Assistant, UserRequest, connect::handle_connection},
        error::Error,
    },
};
use agent_client_protocol::ByteStreams;
use async_channel::Receiver;
use child::Child;
use futures::{AsyncBufReadExt, StreamExt};
use std::io::Write;
use std::sync::Arc;
use tracing::{info, instrument, trace, warn};

async fn read_stderr_lines<R: futures::AsyncBufRead + Unpin, W: Write>(
    reader: R,
    mut writer: W,
    agent_name: &str,
) {
    let mut lines = reader.lines();
    while let Some(line) = lines.next().await {
        match line {
            Ok(line) if !line.is_empty() => {
                writeln!(writer, "[hermes] [stderr] {}: {}", agent_name, line).ok();
            }
            Err(e) => {
                writeln!(
                    writer,
                    "[hermes] stderr read error for '{}': {}",
                    agent_name, e
                )
                .ok();
                break;
            }
            _ => {}
        }
    }
    writeln!(
        writer,
        "[hermes] stderr reader finished for '{}' (EOF)",
        agent_name
    )
    .ok();
}

#[instrument(level = "trace", skip(client, receiver, stdio))]
pub async fn connect(
    client: Arc<Handler>,
    agent: Assistant,
    receiver: Receiver<UserRequest>,
    stdio: Arc<Child>,
) -> Result<(), Error> {
    trace!("Starting stdio connection for '{}'", agent);
    stdio.initialize(&mut agent.command().await?).await?;

    let outgoing = stdio
        .take_stdin()
        .await
        .ok_or_else(|| Error::Connection("Failed to take stdin".to_string()))?;

    let incoming = stdio
        .take_stdout()
        .await
        .ok_or_else(|| Error::Connection("Failed to take stdout".to_string()))?;

    let stderr = stdio.take_stderr().await;
    let agent_name = agent.to_string();
    if let Some(stderr) = stderr {
        std::thread::spawn(move || {
            smol::block_on(async {
                read_stderr_lines(
                    futures::io::BufReader::new(stderr),
                    std::io::stderr(),
                    &agent_name,
                )
                .await;
            });
        });
    } else {
        eprintln!("[hermes] no stderr handle available for '{}'", agent_name);
    }

    let result = handle_connection(
        client,
        agent.clone(),
        receiver,
        ByteStreams::new(outgoing, incoming),
    )
    .await;

    // Reap the child so its exit status is logged. Best-effort: if the wait
    // fails we still propagate the connection result.
    match stdio.wait().await {
        Ok(status) => info!("Disconnected from '{}' with exit status: {}", agent, status),
        Err(e) => warn!("Failed to reap child process for '{}': {}", agent, e),
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures::io::{BufReader, Cursor};
    use pretty_assertions::assert_eq;
    use std::io::ErrorKind;

    fn run_test<F, Fut>(f: F)
    where
        F: FnOnce() -> Fut,
        Fut: std::future::Future<Output = ()>,
    {
        let executor = smol::LocalExecutor::new();
        smol::block_on(executor.run(f()));
    }

    struct FailOnceRead {
        first: bool,
    }

    impl futures::AsyncRead for FailOnceRead {
        fn poll_read(
            mut self: std::pin::Pin<&mut Self>,
            _cx: &mut std::task::Context<'_>,
            buf: &mut [u8],
        ) -> std::task::Poll<std::io::Result<usize>> {
            if self.first {
                self.first = false;
                let data = b"ok\n";
                let len = std::cmp::min(buf.len(), data.len());
                buf[..len].copy_from_slice(data);
                std::task::Poll::Ready(Ok(len))
            } else {
                std::task::Poll::Ready(Err(std::io::Error::new(
                    ErrorKind::Other,
                    "mock read error",
                )))
            }
        }
    }

    #[test]
    fn stderr_lines_reads_all_lines() {
        run_test(|| async {
            let reader = Cursor::new(b"line1\nline2\nline3\n");
            let mut writer = Vec::new();
            read_stderr_lines(reader, &mut writer, "test-agent").await;
            let output = String::from_utf8(writer).unwrap();
            assert_eq!(
                output,
                "[hermes] [stderr] test-agent: line1\n\
                 [hermes] [stderr] test-agent: line2\n\
                 [hermes] [stderr] test-agent: line3\n\
                 [hermes] stderr reader finished for 'test-agent' (EOF)\n"
            );
        });
    }

    #[test]
    fn stderr_lines_skips_empty_lines() {
        run_test(|| async {
            let reader = Cursor::new(b"\n\nhello\n\n");
            let mut writer = Vec::new();
            read_stderr_lines(reader, &mut writer, "test-agent").await;
            let output = String::from_utf8(writer).unwrap();
            assert_eq!(
                output,
                "[hermes] [stderr] test-agent: hello\n\
                 [hermes] stderr reader finished for 'test-agent' (EOF)\n"
            );
        });
    }

    #[test]
    fn stderr_lines_breaks_on_read_error() {
        run_test(|| async {
            let reader = BufReader::new(FailOnceRead { first: true });
            let mut writer = Vec::new();
            read_stderr_lines(reader, &mut writer, "test-agent").await;
            let output = String::from_utf8(writer).unwrap();
            assert_eq!(
                output,
                "[hermes] [stderr] test-agent: ok\n\
                 [hermes] stderr read error for 'test-agent': mock read error\n\
                 [hermes] stderr reader finished for 'test-agent' (EOF)\n"
            );
        });
    }

    #[test]
    fn stderr_lines_logs_eof_on_complete() {
        run_test(|| async {
            let reader = Cursor::new(b"hello\n");
            let mut writer = Vec::new();
            read_stderr_lines(reader, &mut writer, "test-agent").await;
            let output = String::from_utf8(writer).unwrap();
            assert_eq!(
                output,
                "[hermes] [stderr] test-agent: hello\n\
                 [hermes] stderr reader finished for 'test-agent' (EOF)\n"
            );
        });
    }

    #[test]
    fn stderr_lines_handles_no_trailing_newline() {
        run_test(|| async {
            let reader = Cursor::new(b"partial line");
            let mut writer = Vec::new();
            read_stderr_lines(reader, &mut writer, "test-agent").await;
            let output = String::from_utf8(writer).unwrap();
            assert_eq!(
                output,
                "[hermes] [stderr] test-agent: partial line\n\
                 [hermes] stderr reader finished for 'test-agent' (EOF)\n"
            );
        });
    }
}
