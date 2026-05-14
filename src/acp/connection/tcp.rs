//! TCP transport: connect to a remote ACP agent and drive an ACP connection
//! over the socket.
//!
//! Like the `stdio` module, this owns the per-protocol orchestration. The ACP
//! `Client.builder()` plumbing is shared via
//! [`crate::acp::connection::connect::run_connection`].

use crate::{
    Handler,
    acp::{
        connection::{Assistant, UserRequest, connect::handle_connection},
        error::Error,
    },
};
use agent_client_protocol::ByteStreams;
use async_channel::Receiver;
use async_net::TcpStream;
use futures_lite::io::split;
use std::sync::Arc;
use tracing::{debug, error, info, instrument};

#[instrument(level = "trace", skip(client, receiver))]
pub async fn connect(
    client: Arc<Handler>,
    agent: Assistant,
    receiver: Receiver<UserRequest>,
) -> Result<(), Error> {
    let (host, port) = match &agent {
        Assistant::CustomUrl { host, port, .. } => (host.clone(), *port),
        other => {
            error!("Unsupported agent type for tcp connection: {}", other);
            return Err(Error::Connection(format!(
                "TCP protocol requires a CustomUrl assistant, got {}",
                other
            )));
        }
    };

    let address = format!("{}:{}", host, port);
    debug!("Connecting to agent at {}", address);

    let stream = TcpStream::connect(&address)
        .await
        .map_err(|e| Error::Connection(format!("Failed to connect to {}: {}", address, e)))?;

    info!("Connected to agent '{}' via tcp at {}", agent, address);

    let (reader, writer) = split(stream);

    let result = handle_connection(
        client,
        agent.clone(),
        receiver,
        ByteStreams::new(writer, reader),
    )
    .await;

    info!("Disconnected from '{}' via tcp", agent);
    result
}
