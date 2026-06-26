#![allow(private_interfaces)]

// MockAgent for e2e tests, rewritten for agent-client-protocol 0.11.
//
// In 0.10.x this was an `impl Agent for MockAgent` block driven by
// `AgentSideConnection::new(...)`. In 0.11 there is no `Agent` trait; handlers
// are closures registered on `Agent.builder()`. The MockConfig + AgentToConnection
// channel architecture stays the same so existing tests don't change.

use agent_client_protocol::{
    self as acp, Agent, ByteStreams, ConnectionTo, Responder, on_receive_notification,
    on_receive_request,
    schema::v1::{
        AuthenticateRequest, AuthenticateResponse, CancelNotification, CloseSessionRequest,
        CloseSessionResponse, ContentBlock, ContentChunk, CreateTerminalRequest,
        CreateTerminalResponse, DeleteSessionRequest, DeleteSessionResponse, InitializeRequest,
        InitializeResponse, ListSessionsRequest, ListSessionsResponse, LoadSessionRequest,
        LoadSessionResponse, LogoutRequest, LogoutResponse, NewSessionRequest, NewSessionResponse,
        PromptRequest, PromptResponse, ReadTextFileRequest, ReadTextFileResponse,
        ReleaseTerminalRequest, ReleaseTerminalResponse, RequestPermissionOutcome,
        RequestPermissionRequest, ResumeSessionRequest, ResumeSessionResponse, SessionNotification,
        SessionUpdate, SetSessionConfigOptionRequest, SetSessionConfigOptionResponse,
        SetSessionModeRequest, SetSessionModeResponse, StopReason, TerminalOutputRequest,
        TerminalOutputResponse, TextContent, WaitForTerminalExitRequest,
        WaitForTerminalExitResponse, WriteTextFileRequest, WriteTextFileResponse,
    },
};
use async_channel::{Receiver, Sender, bounded, unbounded};
use async_io::{Async, Timer};
use futures::future::{Either, select};
use futures::io::AsyncReadExt;
use std::rc::Rc;
use std::sync::{Arc, Mutex};
use tracing::{error, info};

use super::mock_agent_handle::MockAgentHandle;
use super::mock_config::{MockConfig, generate_session_id};

/// Internal error code for mock agent errors (JSON-RPC internal error)
const INTERNAL_ERROR_CODE: i32 = -32603;

/// Create an internal error with a message
fn internal_error(message: impl Into<String>) -> acp::Error {
    acp::Error::new(INTERNAL_ERROR_CODE, message)
}

/// Messages sent from request handlers to the connection-management task.
/// The connection-management task owns the `ConnectionTo<Client>` and forwards
/// these into outbound `cx.send_request(...)` / `cx.send_notification(...)` calls.
pub(crate) enum AgentToConnection {
    /// Send a session notification to Hermes
    SessionNotification(SessionNotification, Sender<()>),
    /// Send a permission request to Hermes and return the outcome
    PermissionRequest(RequestPermissionRequest, Sender<RequestPermissionOutcome>),
    /// Send a terminal creation request to Hermes and return the response
    CreateTerminal(
        CreateTerminalRequest,
        Sender<acp::Result<CreateTerminalResponse>>,
    ),
    /// Send a terminal output request to Hermes and return the response
    TerminalOutput(
        TerminalOutputRequest,
        Sender<acp::Result<TerminalOutputResponse>>,
    ),
    /// Send a wait for terminal exit request to Hermes and return the response
    WaitForTerminalExit(
        WaitForTerminalExitRequest,
        Sender<acp::Result<WaitForTerminalExitResponse>>,
    ),
    /// Send a read text file request to Hermes and return the response
    ReadTextFile(
        ReadTextFileRequest,
        Sender<acp::Result<ReadTextFileResponse>>,
    ),
    /// Send a write text file request to Hermes and return the response
    WriteTextFile(
        WriteTextFileRequest,
        Sender<acp::Result<WriteTextFileResponse>>,
    ),
    /// Send a release terminal request to Hermes and return the response
    ReleaseTerminal(
        ReleaseTerminalRequest,
        Sender<acp::Result<ReleaseTerminalResponse>>,
    ),
}

/// Opaque receiver type passed from `MockAgent::new()` to `MockAgent::start()`.
pub type MockAgentReceiver = Receiver<AgentToConnection>;

/// Mock agent state shared with the builder closures.
pub struct MockAgent {
    config: Arc<Mutex<MockConfig>>,
    /// Channel to send messages to the connection-management task
    conn_tx: Sender<AgentToConnection>,
}

impl MockAgent {
    /// Create a new mock agent with default configuration.
    ///
    /// Returns the agent state and the receiver end of the connection channel.
    pub fn new() -> (Self, MockAgentReceiver) {
        let (conn_tx, conn_rx) = unbounded();
        let agent = Self {
            config: Arc::new(Mutex::new(MockConfig::default())),
            conn_tx,
        };
        (agent, conn_rx)
    }

    /// Get access to the configuration for customization
    pub fn config(&self) -> &Arc<Mutex<MockConfig>> {
        &self.config
    }

    /// Start the mock agent on a random available port.
    ///
    /// Spawns a thread with a smol LocalExecutor that:
    /// 1. Accepts one TCP connection
    /// 2. Builds an `Agent.builder()` with handlers that delegate to MockConfig
    /// 3. Uses `with_spawned` to run the AgentToConnection translation task
    /// 4. Drives the connection until the transport closes or shutdown is signaled
    pub fn start(
        agent: MockAgent,
        conn_rx: MockAgentReceiver,
    ) -> Result<MockAgentHandle, std::io::Error> {
        let std_listener = std::net::TcpListener::bind("127.0.0.1:0")?;
        let port = std_listener.local_addr()?.port();

        info!("Mock agent starting on port {}", port);

        let (shutdown_tx, shutdown_rx) = bounded::<()>(1);
        let config_clone = agent.config.clone();
        let MockAgent { config, conn_tx } = agent;

        let thread_handle = std::thread::spawn(move || {
            let executor = Rc::new(smol::LocalExecutor::new());

            let listener = match Async::new(std_listener) {
                Ok(l) => l,
                Err(e) => {
                    error!("Failed to create async listener: {}", e);
                    return;
                }
            };

            smol::block_on(executor.clone().run(async move {
                // Race between accept and shutdown signal
                let accept_fut = async {
                    match listener.accept().await {
                        Ok((stream, addr)) => {
                            info!("Mock agent accepted connection from {}", addr);
                            Some(stream)
                        }
                        Err(e) => {
                            error!("Failed to accept connection: {}", e);
                            None
                        }
                    }
                };

                let shutdown_fut = async {
                    let _ = shutdown_rx.recv().await;
                    info!("Mock agent received shutdown signal before connection");
                };

                match select(Box::pin(accept_fut), Box::pin(shutdown_fut)).await {
                    Either::Left((Some(stream), _)) => {
                        let (read_half, write_half) = stream.split();

                        // The connection-management task receives a ConnectionTo<Client> from the
                        // builder once it starts serving traffic. It pulls AgentToConnection
                        // messages off the channel and translates them to outbound
                        // `cx.send_request(...)` / `cx.send_notification(...)` calls.
                        let connection_task = move |cx: ConnectionTo<acp::Client>| async move {
                            while let Ok(msg) = conn_rx.recv().await {
                                match msg {
                                    AgentToConnection::SessionNotification(notification, tx) => {
                                        if let Err(e) = cx.send_notification(notification) {
                                            error!("Error sending session notification: {}", e);
                                            break;
                                        }
                                        tx.try_send(()).ok();
                                    }
                                    AgentToConnection::PermissionRequest(request, tx) => {
                                        match cx.send_request(request).block_task().await {
                                            Ok(response) => {
                                                tx.try_send(response.outcome).ok();
                                            }
                                            Err(e) => {
                                                error!("Error sending permission request: {}", e);
                                                tx.try_send(RequestPermissionOutcome::Cancelled)
                                                    .ok();
                                            }
                                        }
                                    }
                                    AgentToConnection::CreateTerminal(request, tx) => {
                                        let result = cx.send_request(request).block_task().await;
                                        tx.try_send(result).ok();
                                    }
                                    AgentToConnection::TerminalOutput(request, tx) => {
                                        let result = cx.send_request(request).block_task().await;
                                        tx.try_send(result).ok();
                                    }
                                    AgentToConnection::WaitForTerminalExit(request, tx) => {
                                        let result = cx.send_request(request).block_task().await;
                                        tx.try_send(result).ok();
                                    }
                                    AgentToConnection::ReadTextFile(request, tx) => {
                                        let result = cx.send_request(request).block_task().await;
                                        tx.try_send(result).ok();
                                    }
                                    AgentToConnection::WriteTextFile(request, tx) => {
                                        let result = cx.send_request(request).block_task().await;
                                        tx.try_send(result).ok();
                                    }
                                    AgentToConnection::ReleaseTerminal(request, tx) => {
                                        let result = cx.send_request(request).block_task().await;
                                        tx.try_send(result).ok();
                                    }
                                }
                            }
                            info!("Message handling loop ended");
                            Ok(())
                        };

                        let builder = build_mock_agent_builder(config.clone(), conn_tx.clone())
                            .with_spawned(connection_task);

                        let serve_fut = async move {
                            let result = builder
                                .connect_to(ByteStreams::new(write_half, read_half))
                                .await;
                            if let Err(e) = result {
                                error!("Mock agent connection error: {}", e);
                            }
                            info!("Mock agent connection completed");
                        };

                        let shutdown_wait = async {
                            let _ = shutdown_rx.recv().await;
                            info!("Shutdown received while serving connection");
                        };

                        let _ = select(Box::pin(serve_fut), Box::pin(shutdown_wait)).await;
                    }
                    Either::Left((None, _)) => {
                        info!("Mock agent accept failed, exiting");
                    }
                    Either::Right((_, _)) => {
                        info!("Mock agent shutting down before connection established");
                    }
                }

                info!("Mock agent main task completed");
            }));

            info!("Mock agent thread exiting");
        });

        Ok(MockAgentHandle::new(
            config_clone,
            port,
            thread_handle,
            shutdown_tx,
        ))
    }
}

/// Helper: race a future against a timeout, matching tokio::time::timeout semantics.
async fn timeout<T>(
    duration: std::time::Duration,
    future: impl std::future::Future<Output = T>,
) -> Result<T, acp::Error> {
    match select(Box::pin(future), Box::pin(Timer::after(duration))).await {
        Either::Left((result, _)) => Ok(result),
        Either::Right((_, _)) => Err(internal_error("operation timed out")),
    }
}

/// Helper: race a channel recv against a timeout.
async fn recv_timeout<T>(
    rx: Receiver<T>,
    duration: std::time::Duration,
    timeout_msg: &str,
    closed_msg: &str,
) -> Result<T, acp::Error> {
    match select(Box::pin(rx.recv()), Box::pin(Timer::after(duration))).await {
        Either::Left((Ok(val), _)) => Ok(val),
        Either::Left((Err(_), _)) => Err(internal_error(closed_msg)),
        Either::Right((_, _)) => Err(internal_error(timeout_msg)),
    }
}

/// Build an `Agent.builder()` with every inbound handler registered.
///
/// Each closure clones the shared `config` (and `conn_tx` for the prompt handler)
/// and reads from `MockConfig` to determine the response or to dispatch agent →
/// client messages via the connection-management task.
fn build_mock_agent_builder(
    config: Arc<Mutex<MockConfig>>,
    conn_tx: Sender<AgentToConnection>,
) -> acp::Builder<
    Agent,
    impl acp::HandleDispatchFrom<acp::Client>,
    impl acp::RunWithConnectionTo<acp::Client>,
> {
    Agent
        .builder()
        .name("mock-agent")
        .on_receive_request(
            {
                let config = config.clone();
                move |_req: InitializeRequest,
                      responder: Responder<InitializeResponse>,
                      _cx: ConnectionTo<acp::Client>| {
                    let config = config.clone();
                    async move {
                        let dur = config.lock().unwrap().timeout;
                        let result = timeout(dur, async {
                            Ok::<_, acp::Error>(config.lock().unwrap().initialize_response.clone())
                        })
                        .await
                        .map_err(|_| internal_error("initialize timed out"))
                        .and_then(|r| r);
                        responder.respond_with_result(result)
                    }
                }
            },
            on_receive_request!(),
        )
        .on_receive_request(
            {
                let config = config.clone();
                move |_req: AuthenticateRequest,
                      responder: Responder<AuthenticateResponse>,
                      _cx: ConnectionTo<acp::Client>| {
                    let config = config.clone();
                    async move {
                        let dur = config.lock().unwrap().timeout;
                        let result = timeout(dur, async {
                            Ok::<_, acp::Error>(
                                config.lock().unwrap().authenticate_response.clone(),
                            )
                        })
                        .await
                        .map_err(|_| internal_error("authenticate timed out"))
                        .and_then(|r| r);
                        responder.respond_with_result(result)
                    }
                }
            },
            on_receive_request!(),
        )
        .on_receive_request(
            {
                let config = config.clone();
                move |request: NewSessionRequest,
                      responder: Responder<NewSessionResponse>,
                      _cx: ConnectionTo<acp::Client>| {
                    let config = config.clone();
                    async move {
                        let dur = config.lock().unwrap().timeout;
                        let result = timeout(dur, async {
                            let mut config = config.lock().unwrap();
                            let response = config.new_session_response.clone();
                            config.track_session(response.session_id.clone(), request.cwd.clone());
                            config.new_session_response =
                                NewSessionResponse::new(generate_session_id());
                            Ok::<_, acp::Error>(response)
                        })
                        .await
                        .map_err(|_| internal_error("new_session timed out"))
                        .and_then(|r| r);
                        responder.respond_with_result(result)
                    }
                }
            },
            on_receive_request!(),
        )
        .on_receive_request(
            {
                let config = config.clone();
                let conn_tx = conn_tx.clone();
                // Prompt handling drives a multi-step workflow that calls
                // `cx.send_request(...).block_task().await` on the spawned-task path.
                // If we awaited that workflow inside this handler, the dispatch loop
                // would deadlock (it cannot route the response while the handler is
                // still running). Per the 0.11 migration guide we spawn the work and
                // let it call `responder.respond(...)` when it completes.
                move |request: PromptRequest,
                      responder: Responder<PromptResponse>,
                      cx: ConnectionTo<acp::Client>| {
                    let config = config.clone();
                    let conn_tx = conn_tx.clone();
                    async move {
                        cx.spawn(async move {
                            let result = handle_prompt(config, conn_tx, request).await;
                            let _ = responder.respond_with_result(result);
                            Ok(())
                        })
                    }
                }
            },
            on_receive_request!(),
        )
        .on_receive_notification(
            move |_notification: CancelNotification, _cx: ConnectionTo<acp::Client>| async move {
                Ok(())
            },
            on_receive_notification!(),
        )
        .on_receive_request(
            {
                let config = config.clone();
                move |request: LoadSessionRequest,
                      responder: Responder<LoadSessionResponse>,
                      _cx: ConnectionTo<acp::Client>| {
                    let config = config.clone();
                    async move {
                        let dur = config.lock().unwrap().timeout;
                        let result = timeout(dur, async {
                            let config = config.lock().unwrap();
                            if let Some(ref response) = config.load_session_response {
                                return Ok(response.clone());
                            }
                            if config.sessions.contains_key(&request.session_id) {
                                Ok(LoadSessionResponse::new())
                            } else {
                                Err(internal_error(format!(
                                    "session not found: {}",
                                    request.session_id
                                )))
                            }
                        })
                        .await
                        .map_err(|_| internal_error("load_session timed out"))
                        .and_then(|r| r);
                        responder.respond_with_result(result)
                    }
                }
            },
            on_receive_request!(),
        )
        .on_receive_request(
            {
                let config = config.clone();
                move |request: ResumeSessionRequest,
                      responder: Responder<ResumeSessionResponse>,
                      _cx: ConnectionTo<acp::Client>| {
                    let config = config.clone();
                    async move {
                        let dur = config.lock().unwrap().timeout;
                        let result = timeout(dur, async {
                            let config = config.lock().unwrap();
                            if let Some(ref response) = config.resume_session_response {
                                return Ok(response.clone());
                            }
                            if config.sessions.contains_key(&request.session_id) {
                                Ok(ResumeSessionResponse::new())
                            } else {
                                Err(internal_error(format!(
                                    "session not found: {}",
                                    request.session_id
                                )))
                            }
                        })
                        .await
                        .map_err(|_| internal_error("resume_session timed out"))
                        .and_then(|r| r);
                        responder.respond_with_result(result)
                    }
                }
            },
            on_receive_request!(),
        )
        .on_receive_request(
            {
                let config = config.clone();
                move |_req: SetSessionModeRequest,
                      responder: Responder<SetSessionModeResponse>,
                      _cx: ConnectionTo<acp::Client>| {
                    let config = config.clone();
                    async move {
                        let dur = config.lock().unwrap().timeout;
                        let result = timeout(dur, async {
                            let config = config.lock().unwrap();
                            Ok::<_, acp::Error>(
                                config.set_session_mode_response.clone().unwrap_or_default(),
                            )
                        })
                        .await
                        .map_err(|_| internal_error("set_session_mode timed out"))
                        .and_then(|r| r);
                        responder.respond_with_result(result)
                    }
                }
            },
            on_receive_request!(),
        )
        .on_receive_request(
            {
                let config = config.clone();
                move |_req: SetSessionConfigOptionRequest,
                      responder: Responder<SetSessionConfigOptionResponse>,
                      _cx: ConnectionTo<acp::Client>| {
                    let config = config.clone();
                    async move {
                        let dur = config.lock().unwrap().timeout;
                        let result = timeout(dur, async {
                            let config = config.lock().unwrap();
                            Ok::<_, acp::Error>(
                                config
                                    .set_session_config_option_response
                                    .clone()
                                    .unwrap_or_else(|| SetSessionConfigOptionResponse::new(vec![])),
                            )
                        })
                        .await
                        .map_err(|_| internal_error("set_session_config_option timed out"))
                        .and_then(|r| r);
                        responder.respond_with_result(result)
                    }
                }
            },
            on_receive_request!(),
        )
        .on_receive_request(
            {
                let config = config.clone();
                move |_req: ListSessionsRequest,
                      responder: Responder<ListSessionsResponse>,
                      _cx: ConnectionTo<acp::Client>| {
                    let config = config.clone();
                    async move {
                        let dur = config.lock().unwrap().timeout;
                        let result = timeout(dur, async {
                            let config = config.lock().unwrap();
                            if let Some(ref response) = config.list_sessions_response {
                                return Ok(response.clone());
                            }
                            let sessions: Vec<_> = config.sessions.values().cloned().collect();
                            Ok(ListSessionsResponse::new(sessions))
                        })
                        .await
                        .map_err(|_| internal_error("list_sessions timed out"))
                        .and_then(|r| r);
                        responder.respond_with_result(result)
                    }
                }
            },
            on_receive_request!(),
        )
        .on_receive_request(
            {
                let config = config.clone();
                move |_req: CloseSessionRequest,
                      responder: Responder<CloseSessionResponse>,
                      _cx: ConnectionTo<acp::Client>| {
                    let config = config.clone();
                    async move {
                        let dur = config.lock().unwrap().timeout;
                        let result = timeout(dur, async {
                            let config = config.lock().unwrap();
                            Ok::<_, acp::Error>(
                                config.close_session_response.clone().unwrap_or_default(),
                            )
                        })
                        .await
                        .map_err(|_| internal_error("close_session timed out"))
                        .and_then(|r| r);
                        responder.respond_with_result(result)
                    }
                }
            },
            on_receive_request!(),
        )
        .on_receive_request(
            {
                let config = config.clone();
                move |_req: DeleteSessionRequest,
                      responder: Responder<DeleteSessionResponse>,
                      _cx: ConnectionTo<acp::Client>| {
                    let config = config.clone();
                    async move {
                        let dur = config.lock().unwrap().timeout;
                        let result = timeout(dur, async {
                            let config = config.lock().unwrap();
                            Ok::<_, acp::Error>(
                                config.delete_session_response.clone().unwrap_or_default(),
                            )
                        })
                        .await
                        .map_err(|_| internal_error("delete_session timed out"))
                        .and_then(|r| r);
                        responder.respond_with_result(result)
                    }
                }
            },
            on_receive_request!(),
        )
        .on_receive_request(
            {
                let config = config.clone();
                move |_req: LogoutRequest,
                      responder: Responder<LogoutResponse>,
                      _cx: ConnectionTo<acp::Client>| {
                    let config = config.clone();
                    async move {
                        let dur = config.lock().unwrap().timeout;
                        let result =
                            timeout(dur, async { Ok::<_, acp::Error>(LogoutResponse::new()) })
                                .await
                                .map_err(|_| internal_error("logout timed out"))
                                .and_then(|r| r);
                        responder.respond_with_result(result)
                    }
                }
            },
            on_receive_request!(),
        )
    // NOTE: `ExtRequest`/`ExtNotification` are not `JsonRpcRequest`/`JsonRpcNotification`
    // in agent-client-protocol 0.11. Handling them would require defining a custom
    // typed wrapper or registering a catch-all `on_receive_dispatch`. The previous
    // MockAgent implementation only returned a default response for them, which the
    // existing test suite never exercises meaningfully. They are intentionally omitted
    // here. If a test needs ext support, register a typed wrapper at that point.
}

/// Handle a prompt request. Drives the configured agent → client message flow
/// (permission requests, terminal lifecycle, file ops) via `conn_tx`, then echoes
/// the prompt content back as agent message chunks before returning EndTurn.
async fn handle_prompt(
    config: Arc<Mutex<MockConfig>>,
    conn_tx: Sender<AgentToConnection>,
    request: PromptRequest,
) -> Result<PromptResponse, acp::Error> {
    let dur = config.lock().unwrap().timeout;
    timeout(dur, async {
        // Check if we should request permission
        let permission_request = {
            let config = config.lock().unwrap();
            config.permission_request.clone()
        };

        if let Some(perm_req) = permission_request {
            let (tx, rx) = bounded(1);
            conn_tx
                .send(AgentToConnection::PermissionRequest(perm_req, tx))
                .await
                .map_err(|_| internal_error("failed to send permission request"))?;

            let inner_dur = config.lock().unwrap().timeout;
            let _outcome = recv_timeout(
                rx,
                inner_dur,
                "permission request timed out",
                "permission request channel closed",
            )
            .await?;
        }

        // Check if terminal workflow is configured
        let (create_terminal, send_terminal_output, send_terminal_exit) = {
            let config = config.lock().unwrap();
            (
                config.create_terminal_request.clone(),
                config.terminal_output_request.is_some(),
                config.wait_for_terminal_exit_request.is_some(),
            )
        };

        if let Some(create_req) = create_terminal {
            let (tx, rx) = bounded(1);
            conn_tx
                .send(AgentToConnection::CreateTerminal(create_req, tx))
                .await
                .map_err(|_| internal_error("failed to send create_terminal request"))?;

            let create_response = recv_timeout(
                rx,
                dur,
                "create_terminal timed out",
                "create_terminal channel closed",
            )
            .await?
            .map_err(|e| internal_error(format!("create_terminal failed: {}", e)))?;

            let terminal_id = create_response.terminal_id;

            if send_terminal_output {
                let output_req =
                    TerminalOutputRequest::new(request.session_id.clone(), terminal_id.clone());
                let (tx, rx) = bounded(1);
                conn_tx
                    .send(AgentToConnection::TerminalOutput(output_req, tx))
                    .await
                    .map_err(|_| internal_error("failed to send terminal_output request"))?;

                recv_timeout(
                    rx,
                    dur,
                    "terminal_output timed out",
                    "terminal_output channel closed",
                )
                .await?
                .map_err(|e| internal_error(format!("terminal_output failed: {}", e)))?;
            }

            if send_terminal_exit {
                let exit_req =
                    WaitForTerminalExitRequest::new(request.session_id.clone(), terminal_id);
                let (tx, rx) = bounded(1);
                conn_tx
                    .send(AgentToConnection::WaitForTerminalExit(exit_req, tx))
                    .await
                    .map_err(|_| internal_error("failed to send wait_for_terminal_exit request"))?;

                recv_timeout(
                    rx,
                    dur,
                    "wait_for_terminal_exit timed out",
                    "wait_for_terminal_exit channel closed",
                )
                .await?
                .map_err(|e| internal_error(format!("wait_for_terminal_exit failed: {}", e)))?;
            }
        }

        // Read text file (if configured)
        let read_file_request = {
            let config = config.lock().unwrap();
            config.read_file_request.clone()
        };

        if let Some(read_req) = read_file_request {
            let (tx, rx) = bounded(1);
            conn_tx
                .send(AgentToConnection::ReadTextFile(read_req, tx))
                .await
                .map_err(|_| internal_error("failed to send read_text_file request"))?;

            recv_timeout(
                rx,
                dur,
                "read_text_file timed out",
                "read_text_file channel closed",
            )
            .await?
            .map_err(|e| internal_error(format!("read_text_file failed: {}", e)))?;
        }

        // Write text file (if configured)
        let write_file_request = {
            let config = config.lock().unwrap();
            config.write_file_request.clone()
        };

        if let Some(write_req) = write_file_request {
            let (tx, rx) = bounded(1);
            conn_tx
                .send(AgentToConnection::WriteTextFile(write_req, tx))
                .await
                .map_err(|_| internal_error("failed to send write_text_file request"))?;

            recv_timeout(
                rx,
                dur,
                "write_text_file timed out",
                "write_text_file channel closed",
            )
            .await?
            .map_err(|e| internal_error(format!("write_text_file failed: {}", e)))?;
        }

        // Release terminal (if configured)
        let release_terminal_request = {
            let config = config.lock().unwrap();
            config.release_terminal_request.clone()
        };

        if let Some(release_req) = release_terminal_request {
            let (tx, rx) = bounded(1);
            conn_tx
                .send(AgentToConnection::ReleaseTerminal(release_req, tx))
                .await
                .map_err(|_| internal_error("failed to send release_terminal request"))?;

            recv_timeout(
                rx,
                dur,
                "release_terminal timed out",
                "release_terminal channel closed",
            )
            .await?
            .map_err(|e| internal_error(format!("release_terminal failed: {}", e)))?;
        }

        // Echo back the prompt content as agent message chunks
        for content in &request.prompt {
            let text = match content {
                ContentBlock::Text(text_content) => text_content.text.clone(),
                _ => format!("{:?}", content),
            };

            let notification = SessionNotification::new(
                request.session_id.clone(),
                SessionUpdate::AgentMessageChunk(ContentChunk::new(ContentBlock::Text(
                    TextContent::new(text),
                ))),
            );

            let (tx, rx) = bounded(1);
            if conn_tx
                .send(AgentToConnection::SessionNotification(notification, tx))
                .await
                .is_err()
            {
                break;
            }
            let _ = rx.recv().await;
        }

        Ok(PromptResponse::new(StopReason::EndTurn))
    })
    .await
    .map_err(|_| internal_error("prompt timed out"))?
}
