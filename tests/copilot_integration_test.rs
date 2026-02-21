//! Integration tests for CopilotClient
//!
//! NOTE: These tests are skipped because CopilotClient was never implemented.
//! The codebase contains a `copilot()` function in `apc/agent/copilot.rs` that
//! creates a connection to the GitHub Copilot Language Server, but there is no
//! `CopilotClient` struct with `prompt()`, `new_session()`, and `authenticate()` methods.

#[test]
#[ignore = "CopilotClient not implemented"]
fn test_copilot_client_not_implemented() {
    // This test is ignored because CopilotClient doesn't exist yet.
    // To implement this, create a CopilotClient struct that wraps the copilot()
    // function and provides the prompt, new_session, and authenticate methods.
    panic!("CopilotClient not implemented");
}
