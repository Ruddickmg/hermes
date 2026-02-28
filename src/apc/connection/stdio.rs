use crate::{
    ApcClient,
    apc::{connection::Assistant, error::Error},
};
use agent_client_protocol::{
    Agent, Client, ClientSideConnection, ContentBlock, Implementation, InitializeRequest,
    NewSessionRequest, PromptRequest, ProtocolVersion, TextContent,
};
use std::{ffi::OsStr, process::Stdio, sync::Arc, vec};
use tokio::runtime::Runtime;
use tokio::task::LocalSet;
use tokio_util::compat::{TokioAsyncReadCompatExt, TokioAsyncWriteCompatExt};

pub fn stdio_connection<H, I, S>(
    runtime: &Runtime,
    local_set: &LocalSet,
    client: Arc<ApcClient<H>>,
    command: &str,
    args: I,
) -> Result<ClientSideConnection, Error>
where
    H: Client + 'static,
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
{
    let mut child = runtime
        .block_on(async {
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

    let conne = runtime.block_on(local_set.run_until(async {
        println!("creating connection");
        let (conn, handle_io) =
            agent_client_protocol::ClientSideConnection::new(client, outgoing, incoming, |fut| {
                tokio::task::spawn_local(fut);
            });
        println!("spawinging spawn_local");

        // Handle I/O in the background.
        tokio::task::spawn_local(handle_io);

        println!("initializing");

        let result = conn
            .initialize(
                InitializeRequest::new(ProtocolVersion::V1)
                    .client_info(Implementation::new("neovim", "11.0.6")),
            )
            .await
            .unwrap();

        println!("something! {:?}", result);

        let response = conn
            .new_session(NewSessionRequest::new(std::env::current_dir().unwrap()))
            .await
            .unwrap();

        println!("new session! {:?}", response);

        let content = ContentBlock::Text(TextContent::new("Say Hello!"));
        let res = conn
            .prompt(PromptRequest::new(response.session_id, vec![content]))
            .await
            .unwrap();

        println!("prompt response! {:?}", res);

        conn
    }));

    Ok(conne)
}

pub fn connect<H: Client + 'static>(
    runtime: &Runtime,
    local_set: &LocalSet,
    client: Arc<ApcClient<H>>,
    agent: Assistant,
) -> Result<ClientSideConnection, Error> {
    match agent {
        Assistant::Copilot => stdio_connection(
            runtime,
            local_set,
            client,
            "node",
            ["copilot-language-server", "--acp"],
        ),
        Assistant::Opencode => {
            stdio_connection(
                runtime,
                local_set,
                client,
                "opencode",
                [
                    "acp",
                    // "--cwd",
                ],
            )
        }
    }
}
