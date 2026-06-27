//! Unix domain socket transport: connect to a local ACP agent via filesystem
//! socket and drive an ACP connection over it.
//!
//! Like the `tcp` and `stdio` modules, this owns the per-protocol orchestration.
//! The ACP `Client.builder()` plumbing is shared via
//! [`crate::acp::connection::connect::handle_connection`].

use crate::{
    Handler,
    acp::{
        connection::{Assistant, UserRequest, connect::handle_connection},
        error::Error,
    },
};
use agent_client_protocol::ByteStreams;
use async_channel::Receiver;
use futures_lite::io::split;
use std::sync::Arc;
use tracing::{debug, error, info, instrument};

#[cfg(unix)]
use async_io::Async;

#[instrument(level = "trace", skip(client, receiver))]
#[cfg(unix)]
pub async fn connect(
    client: Arc<Handler>,
    agent: Assistant,
    receiver: Receiver<UserRequest>,
) -> Result<(), Error> {
    let path = match &agent {
        Assistant::Socket { path, .. } => path,
        other => {
            error!("Unsupported agent type for socket connection: {}", other);
            return Err(Error::Connection(format!(
                "Socket protocol requires a Socket assistant, got {}",
                other
            )));
        }
    };

    debug!("Connecting to agent at {}", path);

    let stream = Async::<std::os::unix::net::UnixStream>::connect(path)
        .await
        .map_err(|e| Error::Connection(format!("Failed to connect to {}: {}", path, e)))?;

    info!("Connected to agent '{}' via unix socket at {}", agent, path);

    let (reader, writer) = split(stream);

    let result = handle_connection(
        client,
        agent.clone(),
        receiver,
        ByteStreams::new(writer, reader),
    )
    .await;

    info!("Disconnected from '{}' via unix socket", agent);
    result
}

#[cfg(not(unix))]
pub async fn connect(
    _client: Arc<Handler>,
    _agent: Assistant,
    _receiver: Receiver<UserRequest>,
) -> Result<(), Error> {
    Err(Error::Connection(
        "Unix domain sockets are not supported on this platform".to_string(),
    ))
}
