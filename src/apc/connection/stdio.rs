use crate::{
    apc::{connection::Assistant, error::Error},
    ApcClient,
};
use agent_client_protocol::{Client, ClientSideConnection};
use std::{ffi::OsStr, process::Stdio, sync::Arc};
use tokio_util::compat::{TokioAsyncReadCompatExt, TokioAsyncWriteCompatExt};

pub fn stdio_connection<H, I, S>(
    client: Arc<ApcClient<H>>,
    command: &str,
    args: I,
) -> Result<ClientSideConnection, Error>
where
    H: Client + 'static,
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
{
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|e| Error::Connection(e.to_string()))?;

    let mut child = runtime.block_on(async {
        tokio::process::Command::new(command)
            .args(args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
    })
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

    let (conn, handle_io) = runtime.block_on(async {
        agent_client_protocol::ClientSideConnection::new(client, outgoing, incoming, |fut| {
            tokio::task::spawn_local(fut);
        })
    });

    runtime.spawn(handle_io);

    Ok(conn)
}

pub fn connect<H: Client + 'static>(
    client: Arc<ApcClient<H>>,
    agent: Assistant,
) -> Result<ClientSideConnection, Error> {
    match agent {
        Assistant::Copilot => {
            stdio_connection(client, "node", ["copilot-language-server", "--acp"])
        }
        Assistant::Opencode => stdio_connection(client, "opencode", ["apc"]),
    }
}
