pub mod manager;
pub mod stdio;
pub use manager::*;

use crate::apc::error::Error;
use agent_client_protocol::{AgentCapabilities, CancelNotification, NewSessionRequest};
use std::sync::mpsc::Sender;

#[derive(Debug, Clone)]
pub enum UserRequest {
    Cancel(CancelNotification),
    CreateSession(NewSessionRequest),
}

#[derive(Debug, Clone)]
pub struct Connection {
    sender: Sender<UserRequest>,
    capabilities: Option<AgentCapabilities>,
}

impl Connection {
    pub fn new(sender: Sender<UserRequest>) -> Self {
        Self {
            sender,
            capabilities: None,
        }
    }
    pub fn set_capabilities(&mut self, capabilities: AgentCapabilities) {
        self.capabilities = Some(capabilities);
    }
    pub fn create_session(&self, session: NewSessionRequest) -> Result<(), Error> {
        self.sender
            .send(UserRequest::CreateSession(session))
            .map_err(|e| Error::Internal(e.to_string()))
    }
    pub fn cancel(&self, notification: CancelNotification) -> Result<(), Error> {
        self.sender
            .send(UserRequest::Cancel(notification))
            .map_err(|e| Error::Internal(e.to_string()))
    }
    pub fn prompt(&self) {}
    pub fn authenticate(&self) {}
    pub fn set_config_option(&self) {}
    pub fn set_mode(&self) {}
    pub fn load_session(&self) {}
    pub fn custom_command(&self) {}
    pub fn custom_contification(&self) {}
    pub fn list_sessions(&self) {}
    pub fn fork_session(&self) {}
    pub fn resume_session(&self) {}
}
