//! Integration tests for the APC client
//!
//! These tests verify the complete integration of the APC client
//! with the agent-client-protocol library.

use hermes::client::{ApcClient, ClientConfig};
use agent_client_protocol::{Client, SessionNotification, SessionId, SessionUpdate, ContentChunk, ContentBlock, TextContent};

/// Tests that a client can be created with default configuration
#[test]
fn test_create_client_with_defaults() {
    let config = ClientConfig::default();
    let client = ApcClient::new(config);
    
    // Verify client is created
    assert_eq!(client.config().name, "hermes");
    assert!(client.config().enable_fs);
    assert!(client.config().enable_terminal);
}

/// Tests that a client can be created with custom configuration
#[test]
fn test_create_client_with_custom_config() {
    let config = ClientConfig {
        name: "custom-client".to_string(),
        version: "2.0.0".to_string(),
        enable_fs: false,
        enable_terminal: true,
    };
    
    let client = ApcClient::new(config);
    assert_eq!(client.config().name, "custom-client");
    assert_eq!(client.config().version, "2.0.0");
    assert!(!client.config().enable_fs);
    assert!(client.config().enable_terminal);
}

/// Tests that session notifications are handled correctly
#[tokio::test]
async fn test_handle_session_notification() {
    let client = ApcClient::new(ClientConfig::default());
    
    let notification = SessionNotification::new(
        SessionId::new("test-session-123"),
        SessionUpdate::AgentMessageChunk(ContentChunk::new(
            ContentBlock::Text(TextContent::new("Test message"))
        )),
    );
    
    let result: agent_client_protocol::Result<()> = client.session_notification(notification).await;
    assert!(result.is_ok());
}

/// Tests that multiple session notifications can be handled sequentially
#[tokio::test]
async fn test_handle_multiple_notifications() {
    let client = ApcClient::new(ClientConfig::default());
    
    // Send first notification
    let notif1 = SessionNotification::new(
        SessionId::new("session-1"),
        SessionUpdate::AgentMessageChunk(ContentChunk::new(
            ContentBlock::Text(TextContent::new("First message"))
        )),
    );
    let result1: agent_client_protocol::Result<()> = client.session_notification(notif1).await;
    assert!(result1.is_ok());
    
    // Send second notification
    let notif2 = SessionNotification::new(
        SessionId::new("session-1"),
        SessionUpdate::AgentMessageChunk(ContentChunk::new(
            ContentBlock::Text(TextContent::new("Second message"))
        )),
    );
    let result2: agent_client_protocol::Result<()> = client.session_notification(notif2).await;
    assert!(result2.is_ok());
    
    // Send third notification with different session
    let notif3 = SessionNotification::new(
        SessionId::new("session-2"),
        SessionUpdate::AgentMessageChunk(ContentChunk::new(
            ContentBlock::Text(TextContent::new("Different session"))
        )),
    );
    let result3: agent_client_protocol::Result<()> = client.session_notification(notif3).await;
    assert!(result3.is_ok());
}

/// Tests that different types of session updates can be handled
#[tokio::test]
async fn test_handle_different_update_types() {
    let client = ApcClient::new(ClientConfig::default());
    let session_id = SessionId::new("test-session");
    
    // Test agent message chunk
    let agent_msg = SessionNotification::new(
        session_id.clone(),
        SessionUpdate::AgentMessageChunk(ContentChunk::new(
            ContentBlock::Text(TextContent::new("Agent response"))
        )),
    );
    let result1: agent_client_protocol::Result<()> = client.session_notification(agent_msg).await;
    assert!(result1.is_ok());
    
    // Test user message chunk
    let user_msg = SessionNotification::new(
        session_id.clone(),
        SessionUpdate::UserMessageChunk(ContentChunk::new(
            ContentBlock::Text(TextContent::new("User input"))
        )),
    );
    let result2: agent_client_protocol::Result<()> = client.session_notification(user_msg).await;
    assert!(result2.is_ok());
    
    // Test agent thought chunk
    let thought = SessionNotification::new(
        session_id,
        SessionUpdate::AgentThoughtChunk(ContentChunk::new(
            ContentBlock::Text(TextContent::new("Internal reasoning"))
        )),
    );
    let result3: agent_client_protocol::Result<()> = client.session_notification(thought).await;
    assert!(result3.is_ok());
}

/// Tests that client configuration affects capabilities
#[test]
fn test_client_capabilities() {
    // Client with all capabilities enabled
    let full_config = ClientConfig {
        name: "full".to_string(),
        version: "1.0.0".to_string(),
        enable_fs: true,
        enable_terminal: true,
    };
    let full_client = ApcClient::new(full_config);
    assert!(full_client.config().enable_fs);
    assert!(full_client.config().enable_terminal);
    
    // Client with limited capabilities
    let limited_config = ClientConfig {
        name: "limited".to_string(),
        version: "1.0.0".to_string(),
        enable_fs: false,
        enable_terminal: false,
    };
    let limited_client = ApcClient::new(limited_config);
    assert!(!limited_client.config().enable_fs);
    assert!(!limited_client.config().enable_terminal);
}

/// Tests that client is cloneable for sharing across tasks
#[tokio::test]
async fn test_client_cloneable() {
    let client = ApcClient::new(ClientConfig::default());
    let client_clone = client.clone();
    
    // Both instances should work independently
    let notif = SessionNotification::new(
        SessionId::new("test"),
        SessionUpdate::AgentMessageChunk(ContentChunk::new(
            ContentBlock::Text(TextContent::new("Test"))
        )),
    );
    
    let result1: agent_client_protocol::Result<()> = client.session_notification(notif.clone()).await;
    assert!(result1.is_ok());
    
    let result2: agent_client_protocol::Result<()> = client_clone.session_notification(notif).await;
    assert!(result2.is_ok());
}

