use crate::{
    Handler,
    apc::{
        connection::{Assistant, UserRequest},
        error::Error,
    },
};
use agent_client_protocol::{Agent, Client, Implementation, InitializeRequest, ProtocolVersion};
use std::sync::mpsc::Receiver;
use std::{ffi::OsStr, process::Stdio, sync::Arc};
use tokio_util::compat::{TokioAsyncReadCompatExt, TokioAsyncWriteCompatExt};

pub fn stdio_connection<H, I, S>(
    reciever: Receiver<UserRequest>,
    client: Arc<Handler<H>>,
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

    let _: Result<(), Error> = runtime.block_on(local_set.run_until(async {
        println!("creating connection");
        let (conn, handle_io) = agent_client_protocol::ClientSideConnection::new(
            client.clone(),
            outgoing,
            incoming,
            |fut| {
                tokio::task::spawn_local(fut);
            },
        );

        tokio::task::spawn_local(handle_io);

        let result = conn
            .initialize(
                InitializeRequest::new(ProtocolVersion::V1)
                    .client_info(Implementation::new("neovim", "11.0.6")),
            )
            .await?;

        while let Ok(msg) = reciever.try_recv() {
            match msg {
                UserRequest::Cancel(config) => {
                    conn.cancel(config).await?;
                }
                UserRequest::Prompt(request) => {
                    let response = conn.prompt(request).await?;
                    client.prompted(response).await?;
                }
                UserRequest::Authenticate(request) => {
                    let response = conn.authenticate(request).await?;
                    client.authenticated(response).await?;
                }
                UserRequest::SetConfigOption(request) => {
                    let response = conn.set_session_config_option(request).await?;
                    client.config_option_set(response).await?;
                }
                UserRequest::SetMode(request) => {
                    let response = conn.set_session_mode(request).await?;
                    client.mode_set(response).await?;
                }
                UserRequest::CreateSession(config) => {
                    let response = conn.new_session(config).await?;
                    client.session_created(response).await?;
                }
                UserRequest::LoadSession(request) => {
                    let response = conn.load_session(request).await?;
                    client.session_loaded(response).await?;
                }
                UserRequest::ListSessions(request) => {
                    let response = conn.list_sessions(request).await?;
                    client.sessions_listed(response).await?;
                }
                UserRequest::ForkSession(request) => {
                    let response = conn.fork_session(request).await?;
                    client.session_forked(response).await?;
                }
                UserRequest::ResumeSession(request) => {
                    let response = conn.resume_session(request).await?;
                    client.session_resumed(response).await?;
                }
                UserRequest::SetSessionModel(request) => {
                    let response = conn.set_session_model(request).await?;
                    client.session_model_set(response).await?;
                }
                UserRequest::CustomCommand(request) => {
                    let response = conn.ext_method(request).await?;
                    client.custom_command_executed(response).await?;
                }
                UserRequest::CustomNotification(notification) => {
                    conn.ext_notification(notification).await?;
                }
            };
        }
        Ok(())
    }));
    Ok(())
}

pub fn connect<H: Client + 'static>(
    client: Arc<Handler<H>>,
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
        Assistant::Opencode => stdio_connection(receiver, client, "opencode", ["acp"]),
    }
}
