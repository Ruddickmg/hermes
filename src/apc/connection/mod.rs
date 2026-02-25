pub mod stdio;

use crate::{apc::error::Error, ApcClient};
use agent_client_protocol::{Client, ClientSideConnection};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[derive(PartialEq, Eq, Clone, std::hash::Hash, Serialize, Deserialize, Debug)]
pub enum Protocol {
    Socket,
    Http,
    Stdio,
}

impl std::fmt::Display for Protocol {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Protocol::Socket => write!(f, "socket"),
            Protocol::Http => write!(f, "http"),
            Protocol::Stdio => write!(f, "stdio"),
        }
    }
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

#[derive(PartialEq, Eq, Clone, std::hash::Hash, Serialize, Deserialize, Debug)]
pub enum Assistant {
    Copilot,
    Opencode,
}

impl std::fmt::Display for Assistant {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Assistant::Copilot => write!(f, "copilot"),
            Assistant::Opencode => write!(f, "opencode"),
        }
    }
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
