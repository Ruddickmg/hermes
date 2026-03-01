use std::sync::Arc;

use nvim_oxi::{api::opts::CreateAugroupOpts, lua::Error};

use crate::{
    ClientConfig, Handler, apc::connection::ConnectionManager, nvim::producer::EventHandler,
};

const GROUP: &str = "hermes";

pub struct PluginState {
    pub connection: ConnectionManager<EventHandler>,
}

impl PluginState {
    pub fn new() -> Result<Self, Error> {
        Self::with_config(ClientConfig::default())
    }

    pub fn with_config(config: ClientConfig) -> Result<Self, Error> {
        let client = Arc::new(Handler::new(config, EventHandler::new(GROUP.to_string())));

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
