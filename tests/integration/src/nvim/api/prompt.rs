//! Integration tests for prompt history writes
//!
//! The prompt function writes user messages to local history (in SessionNotification
//! format) before attempting to connect to the agent. These tests verify that
//! history files are written correctly even when the connection fails.

use crate::helpers::mock_runtime;
use async_lock::Mutex;
use hermes::{
    Handler,
    api::Api,
    nvim::{
        api::prompt::{ContentBlockType, PromptContent},
        requests::Requests,
        state::PluginState,
    },
    utilities::detect_project_storage_path,
};
use std::io::{Read, Write};
use std::rc::Rc;
use std::sync::Arc;
use tempfile::TempDir;

fn create_test_api(state: Arc<Mutex<PluginState>>) -> Api {
    let runtime = mock_runtime();
    let requests =
        Rc::new(Requests::new(runtime.clone(), state.clone()).expect("Failed to create requests"));
    let handler = Arc::new(
        Handler::new(state.clone(), runtime.clone(), requests.clone())
            .expect("Failed to create handler"),
    );
    let logger =
        hermes::utilities::logging::Logger::inititalize(&detect_project_storage_path().unwrap())
            .unwrap();
    Api::new(state, logger, handler, requests)
}

fn setup_agent_needing_local_history(state: &Arc<Mutex<PluginState>>) {
    use agent_client_protocol::schema::{
        AgentCapabilities, InitializeResponse, ProtocolVersion, SessionCapabilities,
        SessionResumeCapabilities,
    };
    use hermes::acp::connection::Assistant;

    let agent = Assistant::from("test-agent");
    let session_caps = SessionCapabilities::new().resume(Some(SessionResumeCapabilities::new()));
    let info = InitializeResponse::new(ProtocolVersion::V1).agent_capabilities(
        AgentCapabilities::new()
            .load_session(false)
            .session_capabilities(session_caps),
    );
    smol::block_on(async {
        let mut guard = state.lock().await;
        guard.set_agent_info(agent.clone(), info);
        guard.agent_info.set_agent(agent);
    });
}

fn setup_agent_not_needing_history(state: &Arc<Mutex<PluginState>>) {
    use agent_client_protocol::schema::{AgentCapabilities, InitializeResponse, ProtocolVersion};
    use hermes::acp::connection::Assistant;

    let agent = Assistant::from("test-agent");
    let info = InitializeResponse::new(ProtocolVersion::V1)
        .agent_capabilities(AgentCapabilities::new().load_session(true));
    smol::block_on(async {
        let mut guard = state.lock().await;
        guard.set_agent_info(agent.clone(), info);
        guard.agent_info.set_agent(agent);
    });
}

fn block_on<F>(fut: F) -> F::Output
where
    F: std::future::Future,
{
    futures::executor::block_on(fut)
}

#[nvim_oxi::test]
fn prompt_writes_history_for_single_content_block() -> nvim_oxi::Result<()> {
    let temp_dir = TempDir::new().unwrap();
    let state = Arc::new(Mutex::new(
        PluginState::new().with_storage_path(temp_dir.path().to_path_buf()),
    ));
    setup_agent_needing_local_history(&state);
    let api = create_test_api(state.clone());

    let result = block_on(api.prompt((
        "test-session".to_string(),
        PromptContent::Single(ContentBlockType::Text {
            text: "Hello".to_string(),
        }),
    )));

    assert!(
        result.is_err(),
        "prompt should return error when no connection"
    );

    smol::block_on(async {
        let mut guard = state.lock().await;
        guard.agent_info.history.flush().unwrap();
    });
    std::thread::sleep(std::time::Duration::from_millis(200));

    let history_path = temp_dir
        .path()
        .join("history")
        .join("test-agent")
        .join("test-session.jsonl");
    assert!(history_path.exists(), "History file should exist");

    let mut file = std::fs::File::open(&history_path).unwrap();
    let mut content = String::new();
    file.read_to_string(&mut content).unwrap();
    let lines: Vec<&str> = content.trim().lines().collect();
    assert_eq!(lines.len(), 1, "Should have one history entry");

    let parsed: serde_json::Value = serde_json::from_str(lines[0]).unwrap();
    assert_eq!(
        parsed["update"]["sessionUpdate"], "user_message_chunk",
        "Should store user_message_chunk update"
    );

    Ok(())
}

#[nvim_oxi::test]
fn prompt_writes_multiple_content_blocks_with_same_message_id() -> nvim_oxi::Result<()> {
    let temp_dir = TempDir::new().unwrap();
    let state = Arc::new(Mutex::new(
        PluginState::new().with_storage_path(temp_dir.path().to_path_buf()),
    ));
    setup_agent_needing_local_history(&state);
    let api = create_test_api(state.clone());

    let result = block_on(api.prompt((
        "test-session".to_string(),
        PromptContent::Multiple(vec![
            ContentBlockType::Text {
                text: "First".to_string(),
            },
            ContentBlockType::Text {
                text: "Second".to_string(),
            },
            ContentBlockType::Text {
                text: "Third".to_string(),
            },
        ]),
    )));

    assert!(
        result.is_err(),
        "prompt should return error when no connection"
    );

    smol::block_on(async {
        let mut guard = state.lock().await;
        guard.agent_info.history.flush().unwrap();
    });
    std::thread::sleep(std::time::Duration::from_millis(200));

    let history_path = temp_dir
        .path()
        .join("history")
        .join("test-agent")
        .join("test-session.jsonl");
    assert!(history_path.exists(), "History file should exist");

    let mut file = std::fs::File::open(&history_path).unwrap();
    let mut content = String::new();
    file.read_to_string(&mut content).unwrap();
    let lines: Vec<&str> = content.trim().lines().collect();
    assert_eq!(lines.len(), 3, "Should have three history entries");

    let parsed0: serde_json::Value = serde_json::from_str(lines[0]).unwrap();
    let parsed1: serde_json::Value = serde_json::from_str(lines[1]).unwrap();
    assert_eq!(
        parsed0["update"]["messageId"], parsed1["update"]["messageId"],
        "All entries should share the same messageId"
    );

    Ok(())
}

#[nvim_oxi::test]
fn prompt_skips_history_when_not_needed() -> nvim_oxi::Result<()> {
    let temp_dir = TempDir::new().unwrap();
    let state = Arc::new(Mutex::new(
        PluginState::new().with_storage_path(temp_dir.path().to_path_buf()),
    ));
    setup_agent_not_needing_history(&state);
    let api = create_test_api(state.clone());

    let result = block_on(api.prompt((
        "test-session".to_string(),
        PromptContent::Single(ContentBlockType::Text {
            text: "Hello".to_string(),
        }),
    )));

    assert!(
        result.is_err(),
        "prompt should return error when no connection"
    );

    smol::block_on(async {
        let mut guard = state.lock().await;
        guard.agent_info.history.flush().unwrap();
    });
    std::thread::sleep(std::time::Duration::from_millis(200));

    let history_path = temp_dir
        .path()
        .join("history")
        .join("test-agent")
        .join("test-session.jsonl");
    assert!(
        !history_path.exists(),
        "History file should not exist when needs_local_history is false"
    );

    Ok(())
}

#[nvim_oxi::test]
fn prompt_writes_correct_agent_name_in_path() -> nvim_oxi::Result<()> {
    let temp_dir = TempDir::new().unwrap();
    let state = Arc::new(Mutex::new(
        PluginState::new().with_storage_path(temp_dir.path().to_path_buf()),
    ));
    custom_agent_setup(&state);
    let api = create_test_api(state.clone());

    let result = block_on(api.prompt((
        "test-session".to_string(),
        PromptContent::Single(ContentBlockType::Text {
            text: "Hello".to_string(),
        }),
    )));

    assert!(
        result.is_err(),
        "prompt should return error when no connection"
    );

    smol::block_on(async {
        let mut guard = state.lock().await;
        guard.agent_info.history.flush().unwrap();
    });
    std::thread::sleep(std::time::Duration::from_millis(200));

    let history_path = temp_dir
        .path()
        .join("history")
        .join("custom-agent")
        .join("test-session.jsonl");
    assert!(
        history_path.exists(),
        "History file should use the correct agent name in path"
    );

    Ok(())
}

fn custom_agent_setup(state: &Arc<Mutex<PluginState>>) {
    use agent_client_protocol::schema::{
        AgentCapabilities, InitializeResponse, ProtocolVersion, SessionCapabilities,
        SessionResumeCapabilities,
    };
    use hermes::acp::connection::Assistant;

    let agent = Assistant::from("custom-agent");
    let session_caps = SessionCapabilities::new().resume(Some(SessionResumeCapabilities::new()));
    let info = InitializeResponse::new(ProtocolVersion::V1).agent_capabilities(
        AgentCapabilities::new()
            .load_session(false)
            .session_capabilities(session_caps),
    );
    smol::block_on(async {
        let mut guard = state.lock().await;
        guard.set_agent_info(agent.clone(), info);
        guard.agent_info.set_agent(agent);
    });
}
