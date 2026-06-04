pub mod api;
pub mod autocommands;
pub mod configuration;
pub mod parse;
pub mod requests;
pub mod state;
pub mod terminal;

use crate::{
    Handler,
    acp::registry::Registry,
    api::{DisconnectArgs, Hermes},
    utilities::{Logger, NvimRuntime, detect_project_storage_path},
};
use async_lock::Mutex;
use nvim_oxi::{Dictionary, api::opts::CreateAugroupOpts};
use std::{cell::RefCell, rc::Rc, sync::Arc};
use tracing::error;

pub const GROUP: &str = "hermes";

#[nvim_oxi::plugin]
pub fn hermes() -> nvim_oxi::Result<Dictionary> {
    let storage_path = detect_project_storage_path()?;
    let logger = Logger::inititalize(&storage_path)?;
    let registry = Registry::bundled();
    let nvim_runtime = NvimRuntime::new();
    let plugin_state = Arc::new(Mutex::new(
        state::PluginState::new()
            .with_storage_path(std::path::PathBuf::from(&storage_path))?
            .with_registry(registry.cloned()),
    ));
    let request_handler = Rc::new(requests::Requests::new(
        nvim_runtime.clone(),
        plugin_state.clone(),
    )?);
    let event_handler = Arc::new(Handler::new(
        plugin_state.clone(),
        nvim_runtime.clone(),
        request_handler.clone(),
    )?);
    let api = Rc::new(RefCell::new(api::Api::new(
        plugin_state,
        logger,
        event_handler,
        request_handler,
    )));
    let cloned = api.clone();
    let shutdown_runtime = nvim_runtime.clone();
    let hermes = Hermes::new(nvim_runtime, api)?;

    let group =
        nvim_oxi::api::create_augroup(GROUP, &CreateAugroupOpts::builder().clear(true).build())
            .map_err(|e| {
                nvim_oxi::Error::Api(nvim_oxi::api::Error::Other(format!(
                    "Failed to create autogroup for the '{}' group: {}",
                    GROUP, e
                )))
            })?;

    // Bypass CreateAutocmdOpts builder (version-dependent struct layout) by calling
    // nvim_create_autocmd directly via mlua with a registered Lua function callback.
    let lua = nvim_oxi::mlua::lua();
    let cleanup_cb = lua
        .create_function(move |_lua, _: mlua::Value| match cloned.try_borrow_mut() {
            Ok(mut app) => {
                shutdown_runtime.block_on_primary(async move {
                    app.disconnect(DisconnectArgs::All)
                        .await
                        .inspect_err(|e| error!("Error disconnecting: {:?}", e))
                        .ok();
                });
                Ok(mlua::Value::Boolean(true))
            }
            Err(e) => {
                error!(
                    "An error occurred while disconnecting sessions on exit: {:?}",
                    e
                );
                Ok(mlua::Value::Nil)
            }
        })
        .map_err(|e| {
            nvim_oxi::Error::Api(nvim_oxi::api::Error::Other(format!(
                "Failed to create Lua callback: {}",
                e
            )))
        })?;

    lua.load(
        "local group, cb = ...\n vim.api.nvim_create_autocmd('VimLeavePre', { group = group, callback = cb })",
    )
    .call::<()>((group, cleanup_cb))
    .map_err(|e| {
        nvim_oxi::Error::Api(nvim_oxi::api::Error::Other(format!(
            "Failed to create VimLeavePre autocmd: {}",
            e
        )))
    })?;

    Ok(hermes.into())
}
