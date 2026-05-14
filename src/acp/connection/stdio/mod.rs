pub mod child;

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
use child::Child;
use std::sync::Arc;
use tracing::{info, instrument, trace};

#[instrument(level = "trace", skip(client, receiver, stdio))]
pub async fn stdio_connection(
    receiver: Receiver<UserRequest>,
    client: Arc<Handler>,
    agent: &Assistant,
    stdio: Arc<Child>,
) -> Result<(), Error> {
    stdio.initialize(&mut agent.command()?).await?;

    let stdin = stdio
        .take_stdin()
        .await
        .ok_or_else(|| Error::Connection("Failed to take stdin".to_string()))?;

    let stdout = stdio
        .take_stdout()
        .await
        .ok_or_else(|| Error::Connection("Failed to take stdout".to_string()))?;

    trace!("Starting ACP client connection for '{}'", agent);

    // 0.11 builder: register inbound handlers, then drive the connection.
    // `connect_with` owns the dispatch loop internally; our `main_fn` (run_user_requests)
    // pumps UserRequests off the mpsc receiver and forwards them via `cx.send_request(...)`.
    let agent_for_main = agent.clone();
    let client_for_main = client.clone();
    build_client(client)
        .connect_with(ByteStreams::new(stdin, stdout), async move |cx| {
            run_user_requests(cx, receiver, client_for_main, agent_for_main).await
        })
        .await
        .map_err(|e| Error::Connection(e.to_string()))?;

    // Wait for the child to exit (it may have already exited when the ACP
    // connection closed, or we may need to wait briefly)
    let status = stdio.wait().await?;
    info!("Disconnected from '{}' with exit status: {}", agent, status);
    Ok::<(), Error>(())
}

#[instrument(level = "trace", skip(client, receiver, stdio))]
pub async fn connect(
    client: Arc<Handler>,
    agent: Assistant,
    receiver: Receiver<UserRequest>,
    stdio: Arc<Child>,
) -> Result<(), Error> {
    match agent.clone() {
        Assistant::Copilot
        | Assistant::Opencode
        | Assistant::Gemini
        | Assistant::CustomStdio { .. } => {
            trace!("Starting stdio connection for '{}'", agent);
            stdio_connection(receiver, client, &agent, stdio).await
        }
        _ => {
            tracing::error!("Unsupported agent type for stdio connection: {}", agent);
            Ok(())
        }
    }
}
