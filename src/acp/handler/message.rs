//! Outbound request dispatch loop.
//!
//! Runs as the `main_fn` body passed to `Builder::connect_with(...)`. It pulls
//! `UserRequest`s off the mpsc receiver that Neovim writes into and forwards
//! them to the agent via `cx.send_request(...).block_task().await?` (or
//! `cx.send_notification(...)?` for cancel). Each response is then fanned out
//! back into Neovim by calling the matching `*_response` method on `Handler`,
//! which fires the corresponding autocommand.
//!
//! Inbound traffic (agent → client) is handled separately by the closures
//! registered on `Client.builder()` (see `super::builder::build_client`); the
//! dispatch loop invokes those automatically without involving this function.

use std::sync::Arc;

use async_channel::Receiver;

use agent_client_protocol::{self as acp, ConnectionTo};
use tracing::{debug, error, instrument};

use crate::{
    Handler,
    acp::{
        Result,
        connection::{Assistant, UserRequest},
        error::Error,
    },
};

#[instrument(level = "trace", skip(cx, client))]
async fn dispatch(
    cx: &ConnectionTo<acp::Agent>,
    client: &Arc<Handler>,
    agent: &Assistant,
    msg: UserRequest,
) -> Result<()> {
    match msg {
        UserRequest::Initialize(request) => {
            let response = cx.send_request(request).block_task().await?;
            client.initialized(agent, response).await?;
        }
        UserRequest::Cancel(notification) => {
            cx.send_notification(notification)?;
        }
        UserRequest::Prompt(request) => {
            let response = cx.send_request(request).block_task().await?;
            client.prompted(response).await?;
        }
        UserRequest::Authenticate(request) => {
            let response = cx.send_request(request).block_task().await?;
            client.authenticated(response).await?;
        }
        UserRequest::SetConfigOption(request) => {
            let response = cx.send_request(request).block_task().await?;
            client.config_option_set(response).await?;
        }
        UserRequest::SetMode(request) => {
            let response = cx.send_request(request).block_task().await?;
            client.mode_set(response).await?;
        }
        UserRequest::CreateSession(request) => {
            let response = cx.send_request(request).block_task().await?;
            client.session_created(response).await?;
        }
        UserRequest::LoadSession(request) => {
            let response = cx.send_request(request).block_task().await?;
            client.session_loaded(response).await?;
        }
        UserRequest::ListSessions(request) => {
            let response = cx.send_request(request).block_task().await?;
            client.sessions_listed(response).await?;
        }
        UserRequest::ForkSession(request) => {
            let response = cx.send_request(request).block_task().await?;
            client.session_forked(response).await?;
        }
        UserRequest::ResumeSession(request) => {
            let response = cx.send_request(request).block_task().await?;
            client.session_resumed(response).await?;
        }
        UserRequest::SetSessionModel(request) => {
            let response = cx.send_request(request).block_task().await?;
            client.session_model_set(response).await?;
        }
        UserRequest::Close => return Err(Error::InvalidInput(format!("{:?}", msg))),
    }
    Ok(())
}

/// Drive the connection: read `UserRequest`s from `receiver` and dispatch each
/// one through the ACP connection. Exits cleanly on `UserRequest::Close` or
/// when the channel is closed (signaling disconnect).
#[instrument(level = "trace", skip(cx, receiver, client))]
pub async fn run_user_requests(
    cx: ConnectionTo<acp::Agent>,
    receiver: Receiver<UserRequest>,
    client: Arc<Handler>,
    agent: Assistant,
) -> std::result::Result<(), acp::Error> {
    while let Ok(msg) = receiver.recv().await {
        debug!("Received request from '{}': {:#?}", agent, msg);
        if matches!(msg, UserRequest::Close) {
            debug!("Close requested for '{}'", agent);
            break;
        }
        if let Err(e) = dispatch(&cx, &client, &agent, msg).await {
            error!("Error dispatching user request for '{}': {:?}", agent, e);
        } else {
            debug!("Completed request for '{}'", agent);
        }
    }
    Ok(())
}
