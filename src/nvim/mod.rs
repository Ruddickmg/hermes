pub mod parse;
pub mod producer;

use crate::{
    apc::{
        self,
        client::{ApcClient, ClientConfig},
        connection::{Assistant, ConnectionDetails, ConnectionManager, Protocol},
    },
    nvim::producer::EventHandler,
};
use nvim_oxi::{
    Dictionary, Function,
    api::opts::CreateAugroupOpts,
    lua::{Error, Poppable, Pushable, ffi::State},
};
use std::{
    rc::Rc,
    sync::{Arc, Mutex},
};

const GROUP: &str = "hermes";

impl From<apc::error::Error> for Error {
    fn from(e: apc::error::Error) -> Self {
        Error::RuntimeError(e.to_string())
    }
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
    connection: ConnectionManager<EventHandler>,
}

impl PluginState {
    pub fn new() -> Result<Self, Error> {
        Self::with_config(ClientConfig::default())
    }

    pub fn with_config(config: ClientConfig) -> Result<Self, Error> {
        let client = Arc::new(ApcClient::new(config, EventHandler::new(GROUP.to_string())));

        nvim_oxi::api::create_augroup(GROUP, &CreateAugroupOpts::default()).unwrap();

        Ok(Self {
            connection: ConnectionManager::new(client).map_err(Error::from)?,
        })
    }
}

impl Default for PluginState {
    fn default() -> Self {
        Self::new().unwrap()
    }
}

#[derive(Clone)]
pub struct ConnectionArgs {
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

        let agent = match table.get("agent") {
            Some(v) => {
                let s: String = v.clone().try_into()?;
                Some(Assistant::from(s))
            }
            None => None,
        };

        let protocol = match table.get("protocol") {
            Some(v) => {
                let s: String = v.clone().try_into()?;
                Some(Protocol::from(s))
            }
            None => None,
        };

        Ok(Self { agent, protocol })
    }
}

impl Pushable for ConnectionArgs {
    unsafe fn push(self, state: *mut State) -> Result<i32, Error> {
        let dict = nvim_oxi::Object::from({
            let mut dict = Dictionary::new();

            if let Some(agent) = self.agent {
                dict.insert("agent", agent.to_string());
            }

            if let Some(protocol) = self.protocol {
                dict.insert("protocol", protocol.to_string());
            }

            dict
        });

        // SAFETY: Caller must ensure valid state pointer
        unsafe { dict.push(state) }
    }
}

#[nvim_oxi::plugin]
pub fn api() -> nvim_oxi::Result<Dictionary> {
    let plugin_state = Rc::new(Mutex::new(PluginState::new()?));

    let connect: Function<Option<ConnectionArgs>, Result<(), Error>> =
        Function::from_fn(move |arg: Option<ConnectionArgs>| {
            let details = arg.map(ConnectionDetails::from).unwrap_or_default();
            plugin_state
                .lock()
                .map_err(|e| Error::RuntimeError(e.to_string()))?
                .connection
                .connect(details.clone())
                .map_err(Error::from)?;
            Ok(())
        });

    Ok(Dictionary::from_iter([("connect", connect)]))
}
