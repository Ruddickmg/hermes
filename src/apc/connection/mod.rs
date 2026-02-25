pub mod stdio;

use crate::{ApcClient, apc::error::Error};
use agent_client_protocol::{Client, ClientSideConnection};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[derive(Clone, Serialize, Deserialize)]
pub enum Protocol {
    Socket,
    Http,
    Stdio,
}

impl From<&str> for Protocol {
    fn from(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "socket" => Protocol::Socket,
            "http" => Protocol::Http,
            "stdio" => Protocol::Stdio,
            _ => Protocol::default(), // Default to Stdio if unrecognized
        }
    }
}

impl Default for Protocol {
    fn default() -> Self {
        Protocol::Stdio
    }
}

impl From<String> for Protocol {
    fn from(s: String) -> Self {
        Protocol::from(s.as_str())
    }
}

#[derive(PartialEq, Eq, Clone, std::hash::Hash, Serialize, Deserialize)]
pub enum Assistant {
    Copilot,
    Opencode,
}

impl Default for Assistant {
    fn default() -> Self {
        Assistant::Copilot
    }
}

impl From<&str> for Assistant {
    fn from(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "copilot" => Assistant::Copilot,
            "opencode" => Assistant::Opencode,
            _ => Assistant::default(),
        }
    }
}

impl From<String> for Assistant {
    fn from(s: String) -> Self {
        Assistant::from(s.as_str())
    }
}

#[derive(Clone, Default)]
pub struct ConnectionDetails {
    pub agent: Assistant,
    pub protocol: Protocol,
}

#[derive(Clone)]
pub struct Agent<H: Client> {
    client: Arc<ApcClient<H>>,
}

impl<H: Client + 'static> Agent<H> {
    pub fn new(client: Arc<ApcClient<H>>) -> Self {
        Self { client }
    }
    pub fn connect(
        &self,
        ConnectionDetails { agent, protocol }: ConnectionDetails,
    ) -> Result<ClientSideConnection, Error> {
        match protocol {
            Protocol::Stdio => stdio::connect(self.client.clone(), agent),
            Protocol::Http => unimplemented!(),
            Protocol::Socket => unimplemented!(),
        }
    }
}
