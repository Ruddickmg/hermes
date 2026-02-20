use crate::{ApcClient, apc::error::Error};
use agent_client_protocol::{Client, ClientSideConnection};
use std::{process::Stdio, sync::Arc};
use tokio_util::compat::{TokioAsyncReadCompatExt, TokioAsyncWriteCompatExt};

pub fn copilot<H: Client + 'static>(
    client: Arc<ApcClient<H>>,
) -> Result<ClientSideConnection, Error> {
    let mut child = tokio::process::Command::new("npx")
        .args(["-y", "@github/copilot-language-server@latest", "--acp"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| Error::Connection(e.to_string()))?;

    let outgoing = child
        .stdin
        .take()
        .ok_or_else(|| Error::Connection("Failed to take stdin".to_string()))?
        .compat_write();

    let incoming = child
        .stdout
        .take()
        .ok_or_else(|| Error::Connection("Failed to take stdout".to_string()))?
        .compat();

    let (conn, handle_io) =
        agent_client_protocol::ClientSideConnection::new(client, outgoing, incoming, |fut| {
            tokio::task::spawn_local(fut);
        });

    tokio::spawn(handle_io);

    Ok(conn)
}
