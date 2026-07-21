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
use std::sync::Arc;
use tracing::{info, instrument, trace, warn};

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
                let mut lines = futures::io::BufReader::new(stderr).lines();
                while let Some(line) = lines.next().await {
                    match line {
                        Ok(line) if !line.is_empty() => {
                            eprintln!("[hermes] [stderr] {}: {}", agent_name, line);
                        }
                        Err(e) => {
                            eprintln!("[hermes] stderr read error for '{}': {}", agent_name, e);
                            break;
                        }
                        _ => {}
                    }
                }
                eprintln!("[hermes] stderr reader finished for '{}' (EOF)", agent_name);
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
