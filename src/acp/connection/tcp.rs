use crate::{
    Handler,
    acp::{
        connection::{Assistant, UserRequest},
        error::Error,
        handler::{build_client, message::run_user_requests},
    },
};
use agent_client_protocol::ByteStreams;
use async_channel::Receiver;
use async_net::TcpStream;
use futures_lite::io::split;
use std::sync::Arc;
use tracing::{debug, error, info, instrument, trace};

/// Connect to an agent via TCP.
///
/// # Arguments
/// * `receiver` - Channel to receive user requests (prompts, cancellations, etc.)
/// * `client` - The Handler that processes agent requests
/// * `agent` - Assistant identifier for logging
/// * `host` - Host address (e.g., "localhost")
/// * `port` - TCP port number
#[instrument(level = "trace", skip(client, receiver))]
pub async fn tcp_connection(
    receiver: Receiver<UserRequest>,
    client: Arc<Handler>,
    agent: &Assistant,
    host: &str,
    port: u16,
) -> Result<(), Error> {
    let address = format!("{}:{}", host, port);
    debug!("Connecting to agent at {}", address);

    let stream = TcpStream::connect(&address)
        .await
        .map_err(|e| Error::Connection(format!("Failed to connect to {}: {}", address, e)))?;

    info!("Connected to agent '{}' via tcp at {}", agent, address);

    // Split the stream into read and write halves
    let (reader, writer) = split(stream);

    trace!("Starting ACP client connection for '{}'", agent);

    // 0.11 builder: register inbound handlers, then drive the connection.
    // `connect_with` owns the dispatch loop internally; our `main_fn` (run_user_requests)
    // pumps UserRequests off the mpsc receiver and forwards them via `cx.send_request(...)`.
    let agent_for_main = agent.clone();
    let client_for_main = client.clone();
    build_client(client)
        .connect_with(ByteStreams::new(writer, reader), async move |cx| {
            run_user_requests(cx, receiver, client_for_main, agent_for_main).await
        })
        .await
        .map_err(|e| Error::Connection(e.to_string()))?;

    info!("Disconnected from '{}' via tcp", agent);
    Ok::<(), Error>(())
}

/// Connect to an agent using tcp protocol.
///
/// This is the entry point for tcp-based connections, matching the
/// signature of stdio::connect for consistency.
#[instrument(level = "trace", skip(client, receiver))]
pub async fn connect(
    client: Arc<Handler>,
    agent: Assistant,
    receiver: Receiver<UserRequest>,
) -> Result<(), Error> {
    match agent.clone() {
        Assistant::CustomUrl { host, port, .. } => {
            trace!("Starting custom agent connection: {}", agent);
            tcp_connection(receiver, client, &agent, &host, port).await
        }
        _ => {
            error!("Unsupported agent type for tcp connection: {}", agent);
            Ok(())
        }
    }
}
