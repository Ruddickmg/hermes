//! HTTP transport: connect to a remote ACP agent via WebSocket and drive an
//! ACP connection over the socket.
//!
//! Like the `tcp` and `stdio` modules, this owns the per-protocol orchestration.
//! The ACP `Client.builder()` plumbing is shared via
//! [`crate::acp::connection::connect::handle_connection`].

use crate::{
    Handler,
    acp::{
        connection::{Assistant, UserRequest, connect::handle_connection},
        error::Error,
    },
};
use agent_client_protocol::Lines;
use async_channel::Receiver;
use async_tungstenite::tungstenite::Message;
use futures::{Stream, StreamExt};
use std::sync::Arc;
use tracing::{debug, error, info, instrument};

/// Converts a `WebSocketReceiver` into a `Stream<Item = io::Result<String>>`
/// by extracting text messages and mapping errors.
fn ws_text_stream<S>(
    ws_receiver: async_tungstenite::WebSocketReceiver<S>,
) -> impl Stream<Item = std::io::Result<String>> + Send + 'static
where
    S: futures::AsyncRead + futures::AsyncWrite + Unpin + Send + 'static,
{
    ws_receiver.map(|msg| match msg {
        Ok(Message::Text(text)) => Ok(text.to_string()),
        Ok(Message::Close(_)) => Err(std::io::Error::new(
            std::io::ErrorKind::ConnectionAborted,
            "WebSocket closed",
        )),
        Ok(other) => {
            debug!("Received non-text WebSocket message: {:?}", other);
            Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("Unexpected WebSocket message: {:?}", other),
            ))
        }
        Err(e) => Err(std::io::Error::new(std::io::ErrorKind::Other, e)),
    })
}

#[instrument(level = "trace", skip(client, request_receiver))]
pub async fn connect(
    client: Arc<Handler>,
    agent: Assistant,
    request_receiver: Receiver<UserRequest>,
) -> Result<(), Error> {
    let url = match &agent {
        Assistant::CustomUrl {
            host, port, path, ..
        } => {
            let mut url = format!("ws://{}:{}", host, port);
            if let Some(path) = path {
                url.push_str(path);
            }
            url
        }
        other => {
            error!("Unsupported agent type for http connection: {}", other);
            return Err(Error::Connection(format!(
                "HTTP protocol requires a CustomUrl assistant, got {}",
                other
            )));
        }
    };

    debug!("Connecting to agent at {}", url);

    let (ws_stream, _) = async_tungstenite::smol::connect_async(&url)
        .await
        .map_err(|e| Error::Connection(format!("Failed to connect to {}: {}", url, e)))?;

    info!("Connected to agent '{}' via websocket at {}", agent, url);

    let (ws_sender, ws_receiver) = ws_stream.split();

    // Build a Sink<String, Error = io::Error> from the WebSocketSender
    let outgoing_sink = futures::sink::unfold(ws_sender, |mut sender, text: String| async move {
        sender
            .send(Message::Text(text.into()))
            .await
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
        Ok::<_, std::io::Error>(sender)
    });

    let incoming_stream = ws_text_stream(ws_receiver);

    let lines = Lines::new(outgoing_sink, incoming_stream);

    let result = handle_connection(client, agent.clone(), request_receiver, lines).await;

    info!("Disconnected from '{}' via websocket", agent);
    result
}
