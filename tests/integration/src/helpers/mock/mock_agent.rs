//! Minimal mock ACP agent for integration tests.
//!
//! Starts a lightweight TCP server that accepts one connection and
//! responds to the minimum set of ACP messages needed for testing
//! delete_session: Initialize, DeleteSession, and CancelNotification.

use agent_client_protocol::{
    Agent, ByteStreams, Responder, on_receive_notification, on_receive_request,
    schema::{
        AgentCapabilities, CancelNotification, DeleteSessionRequest, DeleteSessionResponse,
        InitializeRequest, InitializeResponse, ProtocolVersion, SessionCapabilities,
        SessionDeleteCapabilities,
    },
};
use async_io::Async;
use futures::{
    future::{Either, select},
    io::AsyncReadExt,
};
use smol::LocalExecutor;
use std::rc::Rc;
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};
use std::thread::JoinHandle;

pub struct MockAgentHandle {
    pub port: u16,
    handle: Option<JoinHandle<()>>,
    shutdown_tx: async_channel::Sender<()>,
    pub delete_session_received: Arc<AtomicBool>,
}

impl Drop for MockAgentHandle {
    fn drop(&mut self) {
        self.shutdown_tx.try_send(()).ok();
        if let Some(handle) = self.handle.take() {
            let timeout = std::time::Duration::from_secs(5);
            let start = std::time::Instant::now();
            loop {
                if start.elapsed() >= timeout {
                    break;
                }
                if handle.is_finished() {
                    let _ = handle.join();
                    break;
                }
                std::thread::sleep(std::time::Duration::from_millis(50));
            }
        }
    }
}

/// Start a mock ACP agent that responds to Initialize, DeleteSession, and
/// CancelNotification.
pub fn start_mock_agent() -> MockAgentHandle {
    let delete_session_received = Arc::new(AtomicBool::new(false));

    let tracked = delete_session_received.clone();
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    let (shutdown_tx, shutdown_rx) = async_channel::bounded(1);

    let handle = std::thread::spawn(move || {
        let executor = Rc::new(LocalExecutor::new());
        smol::block_on(executor.clone().run(async move {
            let listener = Async::new(listener).unwrap();

            let accept_fut = async {
                match listener.accept().await {
                    Ok((stream, _addr)) => Some(stream),
                    Err(_) => None,
                }
            };

            let shutdown_fut = async {
                let _ = shutdown_rx.recv().await;
            };

            let stream = match select(Box::pin(accept_fut), Box::pin(shutdown_fut)).await {
                Either::Left((Some(stream), _)) => stream,
                _ => return,
            };

            let (read_half, write_half) = stream.split();

            let result = Agent
                .builder()
                .name("integration-test-agent")
                .on_receive_request(
                    {
                        move |_req: InitializeRequest,
                              responder: Responder<InitializeResponse>,
                              _cx: agent_client_protocol::ConnectionTo<
                            agent_client_protocol::Client,
                        >| {
                            async move {
                                responder.respond(
                                    InitializeResponse::new(ProtocolVersion::V1)
                                        .agent_capabilities(
                                            AgentCapabilities::new().session_capabilities(
                                                SessionCapabilities::new()
                                                    .delete(Some(SessionDeleteCapabilities::new())),
                                            ),
                                        ),
                                );
                                Ok(())
                            }
                        }
                    },
                    on_receive_request!(),
                )
                .on_receive_request(
                    {
                        let tracked = tracked.clone();
                        move |_req: DeleteSessionRequest,
                              responder: Responder<DeleteSessionResponse>,
                              _cx: agent_client_protocol::ConnectionTo<
                            agent_client_protocol::Client,
                        >| {
                            tracked.store(true, Ordering::SeqCst);
                            async move {
                                responder.respond(DeleteSessionResponse::new());
                                Ok(())
                            }
                        }
                    },
                    on_receive_request!(),
                )
                .on_receive_notification(
                    {
                        move |_notif: CancelNotification,
                              _cx: agent_client_protocol::ConnectionTo<
                            agent_client_protocol::Client,
                        >| { async move { Ok(()) } }
                    },
                    on_receive_notification!(),
                )
                .connect_to(ByteStreams::new(write_half, read_half))
                .await;

            if let Err(e) = result {
                tracing::error!("Mock agent error: {:?}", e);
            }
        }));
    });

    MockAgentHandle {
        port,
        handle: Some(handle),
        shutdown_tx,
        delete_session_received,
    }
}
