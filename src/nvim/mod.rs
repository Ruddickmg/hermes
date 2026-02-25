pub mod parse;
pub mod producer;

use crate::{
    apc::{
        client::{ApcClient, ClientConfig},
        connection::{Agent, Assistant, ConnectionDetails, Protocol},
    },
    nvim::producer::EventHandler,
};
use agent_client_protocol::ClientSideConnection;
use nvim_oxi::{
    Dictionary, Function,
    api::opts::CreateAugroupOpts,
    lua::{Error, Poppable, ffi::State},
};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

const GROUP: &str = "hermes";

#[derive(Debug)]
pub enum NvimError {
    InitializationError(String),
    NotConnected,
    InvalidConfig(String),
}

/// Neovim plugin state
///
/// This structure maintains the state of the Neovim plugin, including
/// the APC client instance. It uses `Arc` for shared ownership, allowing
/// the client to be accessed from multiple Neovim commands.
///
/// # Examples
///
/// ```
/// use hermes::nvim::PluginState;
///
/// // Create with default configuration
/// let state = PluginState::new();
///
/// // Access the client
/// let client = state.client();
/// ```
pub struct PluginState {
    client: Arc<ApcClient<EventHandler>>,
    agent: Arc<Agent<EventHandler>>,
    connections: HashMap<Assistant, ClientSideConnection>,
}

impl PluginState {
    /// Creates a new plugin state with default configuration
    ///
    /// # Examples
    ///
    /// ```
    /// use hermes::nvim::PluginState;
    ///
    /// let state = PluginState::new();
    /// assert_eq!(state.client().config().name, "hermes");
    /// ```
    pub fn new() -> Self {
        let config = ClientConfig::default();

        nvim_oxi::api::create_augroup(GROUP, &CreateAugroupOpts::default()).unwrap();

        let client = Arc::new(ApcClient::new(config, EventHandler::new(GROUP.to_string())));

        Self {
            client: client.clone(),
            agent: Arc::new(Agent::new(client.clone())),
            connections: HashMap::new(),
        }
    }

    /// Creates a new plugin state with custom configuration
    ///
    /// # Arguments
    ///
    /// * `config` - Client configuration to use
    ///
    /// # Examples
    ///
    /// ```
    /// use hermes::nvim::PluginState;
    /// use hermes::client::ClientConfig;
    ///
    /// let config = ClientConfig {
    ///     name: "custom".to_string(),
    ///     version: "1.0.0".to_string(),
    ///     enable_fs: false,
    ///     enable_terminal: true,
    /// };
    /// let state = PluginState::with_config(config);
    /// assert_eq!(state.client().config().name, "custom");
    /// ```
    pub fn with_config(config: ClientConfig) -> Self {
        let client = Arc::new(ApcClient::new(config, EventHandler::default()));
        Self {
            client: client.clone(),
            agent: Arc::new(Agent::new(client.clone())),
            connections: HashMap::new(),
        }
    }

    pub fn set_connection(&mut self, agent: Assistant, connection: ClientSideConnection) -> &Self {
        self.connections.insert(agent, connection);
        self
    }

    /// Gets a reference to the APC client
    ///
    /// Returns an `Arc` reference to the client, allowing it to be shared
    /// across different parts of the plugin.
    ///
    /// # Examples
    ///
    /// ```
    /// use hermes::nvim::PluginState;
    ///
    /// let state = PluginState::new();
    /// let client = state.client();
    /// assert!(client.config().enable_fs);
    /// ```
    pub fn client(&self) -> &Arc<ApcClient<EventHandler>> {
        &self.client
    }
}

impl Default for PluginState {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Clone)]
struct ConnectionArgs {
    pub agent: Option<Assistant>,
    pub protocol: Option<Protocol>,
}

impl From<ConnectionArgs> for ConnectionDetails {
    fn from(args: ConnectionArgs) -> Self {
        ConnectionDetails {
            agent: args.agent.unwrap_or_default(),
            protocol: args.protocol.unwrap_or_default(),
        }
    }
}

impl Poppable for ConnectionArgs {
    unsafe fn pop(state: *mut State) -> Result<Self, Error> {
        use nvim_oxi::Object;

        let table = unsafe { Dictionary::pop(state)? };

        let agent = table
            .get("agent")
            .map(|v: &Object| unsafe { v.as_nvim_str_unchecked() })
            .map(|s: nvim_oxi::NvimStr| s.to_string())
            .map(Assistant::from);

        let protocol = table
            .get("protocol")
            .map(|v: &Object| unsafe { v.as_nvim_str_unchecked() })
            .map(|s: nvim_oxi::NvimStr| s.to_string())
            .map(Protocol::from);

        Ok(Self { agent, protocol })
    }
}

pub fn setup() -> nvim_oxi::Result<Dictionary> {
    let plugin_state = Arc::new(Mutex::new(PluginState::new()));

    let connect: Function<ConnectionArgs, Result<(), Error>> =
        Function::from_fn::<_, Result<(), Error>>(move |arg: ConnectionArgs| {
            let details = ConnectionDetails::from(arg);
            let connection = plugin_state
                .lock()
                .map_err(|e| Error::RuntimeError(e.to_string()))?
                .agent
                .connect(details.clone())
                .map_err(|e| Error::RuntimeError(e.to_string()))?;
            plugin_state
                .lock()
                .map_err(|e| Error::RuntimeError(e.to_string()))?
                .set_connection(details.agent, connection);
            Ok(())
        });

    Ok(Dictionary::from_iter([("connect", connect)]))
}
