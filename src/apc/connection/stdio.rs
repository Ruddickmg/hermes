use crate::{
    ApcClient,
    apc::{connection::Assistant, error::Error},
};
use agent_client_protocol::{
    Agent, Client, Implementation, InitializeRequest, NewSessionRequest, ProtocolVersion,
};
use std::{ffi::OsStr, process::Stdio, sync::Arc};
use tokio::sync::mpsc::Receiver;
use tokio_util::compat::{TokioAsyncReadCompatExt, TokioAsyncWriteCompatExt};

#[derive(Debug)]
pub enum UserRequest {
    CreateSession,
}

pub fn stdio_connection<H, I, S>(
    mut reciever: Receiver<UserRequest>,
    client: Arc<ApcClient<H>>,
    command: &str,
    args: I,
) -> Result<(), Error>
where
    H: Client + 'static,
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
{
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|e| Error::Connection(e.to_string()))?;
    let local_set = tokio::task::LocalSet::new();

    let mut child = tokio::process::Command::new(command)
        .args(args)
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

    runtime.block_on(local_set.run_until(async {
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
        while let Ok(msg) = reciever.try_recv() {
            println!("got a message from the channel! {:?}", msg);
            match msg {
                UserRequest::CreateSession => {
                    let response = conn
                        .new_session(NewSessionRequest::new(
                            // TODO: find the project root
                            std::env::current_dir()
                                .map_err(|e| Error::Internal(e.to_string()))
                                .unwrap_or(".".to_string().into()),
                        ))
                        .await
                        .unwrap();

                    println!("new session! {:?}", response);
                }
            }
        }

        // let response = conn
        //     .new_session(NewSessionRequest::new(std::env::current_dir().unwrap()))
        //     .await
        //     .unwrap();
        //
        // println!("new session! {:?}", response);
        //
        // let content = ContentBlock::Text(TextContent::new("Say Hello!"));
        // let res = conn
        //     .prompt(PromptRequest::new(response.session_id, vec![content]))
        //     .await
        //     .unwrap();
        //
        // println!("prompt response! {:?}", res);
    }));

    Ok(())
}

pub fn connect<H: Client + 'static>(
    client: Arc<ApcClient<H>>,
    agent: Assistant,
    receiver: Receiver<UserRequest>,
) -> Result<(), Error> {
    match agent {
        Assistant::Copilot => stdio_connection(
            receiver,
            client,
            "node",
            ["copilot-language-server", "--acp"],
        ),
        Assistant::Opencode => {
            stdio_connection(
                receiver,
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
