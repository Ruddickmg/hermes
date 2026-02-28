pub mod stdio;

use crate::{ApcClient, apc::error::Error};
use agent_client_protocol::{Client, ClientSideConnection};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::Arc;
use tokio::runtime::Runtime;
use tokio::task::LocalSet;

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

#[derive(Debug, Clone, Default)]
pub struct ConnectionDetails {
    pub agent: Assistant,
    pub protocol: Protocol,
}

#[derive(Clone)]
pub struct ConnectionManager<H: Client> {
    connection: HashMap<Assistant, Rc<ClientSideConnection>>,
    handler: Arc<ApcClient<H>>,
    runtime: Arc<Runtime>,
    local: Rc<LocalSet>,
}

impl<H: Client + 'static> ConnectionManager<H> {
    pub fn new(client: Arc<ApcClient<H>>) -> Result<Self, Error> {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .map_err(|e| Error::Internal(e.to_string()))?;

        let local_set = tokio::task::LocalSet::new();

        Ok(Self {
            handler: client,
            connection: HashMap::new(),
            runtime: Arc::new(runtime),
            local: Rc::new(local_set),
        })
    }

    fn add_connection(&mut self, agent: Assistant, connection: ClientSideConnection) {
        self.connection.insert(agent, Rc::new(connection));
    }

    pub fn get_connection(&self, agent: &Assistant) -> Option<Rc<ClientSideConnection>> {
        self.connection.get(agent).cloned()
    }

    pub fn connect(
        &mut self,
        ConnectionDetails { agent, protocol }: ConnectionDetails,
    ) -> Result<Rc<ClientSideConnection>, Error> {
        let connection = match protocol {
            Protocol::Stdio => stdio::connect(
                &self.runtime,
                &self.local,
                self.handler.clone(),
                agent.clone(),
            ),
            Protocol::Http => unimplemented!(),
            Protocol::Socket => unimplemented!(),
        }
        .map_err(|e| Error::Connection(e.to_string()))?;
        self.add_connection(agent.clone(), connection);
        self.get_connection(&agent).ok_or_else(|| {
            Error::Connection("Failed to retrieve connection after creation".to_string())
        })
    }
}
