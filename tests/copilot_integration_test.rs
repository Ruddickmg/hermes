//! Integration tests for CopilotClient
//!
//! These tests connect to the real GitHub Copilot Language Server.
//! They will fail if Copilot is not installed or not authenticated.

use agent_client_protocol::{
    Agent, AuthenticateRequest, NewSessionRequest, PromptRequest, SessionId,
};
use std::path::PathBuf;
use hermes::agent::copilot::CopilotClient;

fn is_copilot_available() -> bool {
    std::process::Command::new("npx")
        .args(["-y", "@github/copilot-language-server@latest", "--version"])
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

#[tokio::test]
async fn test_methods_fail_without_init() {
    let client = CopilotClient::default();

    // Test prompt without initialization
    let prompt_result = client.prompt(PromptRequest::new(SessionId::new("test-session"), vec![])).await;
    assert!(prompt_result.is_err());

    // Test new_session without initialization  
    let session_result = client.new_session(NewSessionRequest::new(PathBuf::from("/tmp"))).await;
    assert!(session_result.is_err());

    // Test authenticate without initialization
    let auth_result = client.authenticate(agent_client_protocol::AuthenticateRequest::new("github")).await;
    assert!(auth_result.is_err());
}

// Note: The following tests require Copilot to be installed and would need
// to run within a LocalSet to handle the spawn_local requirement.
// They are kept here for reference but may fail without proper setup.

// #[tokio::test]
// async fn test_initialize_with_copilot() {
//     if !is_copilot_available() {
//         panic!("Copilot Language Server not available");
//     }
//     let client = CopilotClient::default();
//     let result = client.initialize(InitializeRequest::new(ProtocolVersion::V1)).await;
//     assert!(result.is_ok());
// }
