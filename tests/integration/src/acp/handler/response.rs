use crate::helpers::{MockRequestHandler, mock_runtime};
use agent_client_protocol::schema::{
    AuthenticateResponse, CloseSessionResponse, ForkSessionResponse, ListSessionsResponse,
    ResumeSessionResponse,
};
use async_lock::Mutex;
use hermes::acp::handler::Handler;
use hermes::acp::session_info::SessionDetails;
use hermes::nvim::state::PluginState;
use std::rc::Rc;
use std::sync::Arc;

fn create_handler() -> Handler {
    Handler::new(
        Arc::new(Mutex::new(PluginState::default())),
        mock_runtime(),
        Rc::new(MockRequestHandler::new()),
    )
    .expect("Handler creation should succeed")
}

#[nvim_oxi::test]
fn authenticated_succeeds() -> nvim_oxi::Result<()> {
    let handler = create_handler();
    let response = AuthenticateResponse::default();
    let result = smol::block_on(handler.authenticated(response));
    assert!(result.is_ok(), "authenticated should succeed");
    Ok(())
}

#[nvim_oxi::test]
fn custom_command_executed_succeeds() -> nvim_oxi::Result<()> {
    let handler = create_handler();
    let raw = serde_json::value::RawValue::from_string("{}".to_string())
        .map(std::sync::Arc::from)
        .expect("RawValue creation should succeed");
    let response = agent_client_protocol::schema::ExtResponse::new(raw);
    let result = smol::block_on(handler.custom_command_executed(response));
    assert!(result.is_ok(), "custom_command_executed should succeed");
    Ok(())
}

#[nvim_oxi::test]
fn sessions_listed_succeeds() -> nvim_oxi::Result<()> {
    let handler = create_handler();
    let response = ListSessionsResponse::new(vec![]);
    let result = smol::block_on(handler.sessions_listed(response));
    assert!(result.is_ok(), "sessions_listed should succeed");
    Ok(())
}

#[nvim_oxi::test]
fn session_forked_succeeds() -> nvim_oxi::Result<()> {
    let handler = create_handler();
    let response = ForkSessionResponse::new("forked-session");
    let result = smol::block_on(handler.session_forked(response));
    assert!(result.is_ok(), "session_forked should succeed");
    Ok(())
}

#[nvim_oxi::test]
fn session_resumed_succeeds() -> nvim_oxi::Result<()> {
    let handler = create_handler();
    let response = ResumeSessionResponse::default();
    let result = smol::block_on(handler.session_resumed(response));
    assert!(result.is_ok(), "session_resumed should succeed");
    Ok(())
}

#[nvim_oxi::test]
fn session_closed_succeeds() -> nvim_oxi::Result<()> {
    let handler = create_handler();
    let session_id = String::from("test-session");
    let response = CloseSessionResponse::default();
    let result = smol::block_on(handler.session_closed(session_id, response));
    assert!(result.is_ok(), "session_closed should succeed");
    Ok(())
}

#[nvim_oxi::test]
fn session_closed_removes_session_info() -> nvim_oxi::Result<()> {
    let state = Arc::new(Mutex::new(PluginState::default()));
    let handler = Handler::new(
        state.clone(),
        mock_runtime(),
        Rc::new(MockRequestHandler::new()),
    )
    .expect("Handler creation should succeed");

    let session_id = String::from("test-session");
    // Insert session info using SessionDetails
    {
        let mut locked = smol::block_on(state.lock());
        locked
            .session_info
            .insert(session_id.clone(), SessionDetails::default());
        locked
            .prompt
            .insert(session_id.clone(), "stale-prompt".to_string());
    }

    let response = CloseSessionResponse::default();
    smol::block_on(handler.session_closed(session_id.clone(), response))
        .map_err(|e| nvim_oxi::api::Error::Other(e.to_string()))?;

    let locked = smol::block_on(state.lock());
    assert!(
        !locked.session_info.contains_key(&session_id),
        "session_info should be removed after close"
    );
    Ok(())
}

#[nvim_oxi::test]
fn session_closed_removes_prompt() -> nvim_oxi::Result<()> {
    let state = Arc::new(Mutex::new(PluginState::default()));
    let handler = Handler::new(
        state.clone(),
        mock_runtime(),
        Rc::new(MockRequestHandler::new()),
    )
    .expect("Handler creation should succeed");

    let session_id = String::from("test-session");
    // Insert prompt data
    {
        let mut locked = smol::block_on(state.lock());
        locked
            .prompt
            .insert(session_id.clone(), "stale-prompt".to_string());
    }

    let response = CloseSessionResponse::default();
    smol::block_on(handler.session_closed(session_id.clone(), response))
        .map_err(|e| nvim_oxi::api::Error::Other(e.to_string()))?;

    let locked = smol::block_on(state.lock());
    assert!(
        !locked.prompt.contains_key(&session_id),
        "prompt should be removed after close"
    );
    Ok(())
}

#[nvim_oxi::test]
fn session_notification_unknown_update_returns_method_not_found() -> nvim_oxi::Result<()> {
    let handler = create_handler();
    let info = agent_client_protocol::schema::SessionInfoUpdate::new();
    let notification = agent_client_protocol::schema::SessionNotification::new(
        "test-session",
        agent_client_protocol::schema::SessionUpdate::SessionInfoUpdate(info),
    );
    let result = smol::block_on(handler.session_notification(notification));
    assert_eq!(
        result.unwrap_err(),
        agent_client_protocol::Error::method_not_found(),
        "Unhandled SessionUpdate variant should return MethodNotFound"
    );
    Ok(())
}
