use hermes::nvim::autocommands::Commands;
use nvim_oxi::api::{self};
use nvim_oxi::{Array, Object, serde::Deserializer};
use serde::de::DeserializeOwned;
use std::{
    fmt::Debug,
    sync::atomic::{AtomicU64, Ordering},
    time::{Duration, Instant},
};
use tracing::error;

pub fn nvim_object_to_struct<T>(obj: Object) -> Result<T, nvim_oxi::Error>
where
    T: DeserializeOwned,
{
    T::deserialize(Deserializer::new(obj))
        .map_err(|e| nvim_oxi::Error::Api(nvim_oxi::api::Error::Other(e.to_string())))
}

static LISTENER_ID: AtomicU64 = AtomicU64::new(0);

pub fn listen_for_autocommand<T>(
    autocommand: Commands,
) -> Box<dyn Fn(Duration) -> Result<T, nvim_oxi::Error>>
where
    T: Debug + DeserializeOwned + Send + Clone + 'static,
{
    let pattern = autocommand.to_string();
    let id = LISTENER_ID.fetch_add(1, Ordering::Relaxed);
    let data_var = format!("_hermes_e2e_data_{}", id);

    // Use nvim_exec_lua to avoid CreateAutocmdOpts builder mask bug:
    // neovim-0-12 feature renames "buffer" -> "buf", shifting hashy_hash
    // ordering so that .group() sets the bit Neovim interprets as "buffer".
    let lua_code = format!(
        r#"vim.api.nvim_create_autocmd("User",{{pattern={{"{}"}},group="hermes",callback=function(a)_G["{}"]=a.data end}})"#,
        pattern, data_var
    );

    let args: Array = (Object::from(lua_code.as_str()), Object::from(Array::new())).into();
    api::call_function::<Array, Object>("nvim_exec_lua", args)
        .expect("Failed to create autocommand listener via nvim_exec_lua");

    Box::new(move |duration| {
        let start = Instant::now();
        loop {
            let lua_get = format!(r#"return _G["{}"]"#, data_var);
            let get_args: Array =
                (Object::from(lua_get.as_str()), Object::from(Array::new())).into();
            let data: Object = match api::call_function("nvim_exec_lua", get_args) {
                Ok(d) => d,
                Err(e) => {
                    error!("Error polling autocmd data: {:#?}", e);
                    Object::nil()
                }
            };

            if !data.is_nil() {
                let lua_clear = format!(r#"_G["{}"] = nil"#, data_var);
                let clear_args: Array =
                    (Object::from(lua_clear.as_str()), Object::from(Array::new())).into();
                let _: Result<Object, _> = api::call_function("nvim_exec_lua", clear_args);
                return nvim_object_to_struct(data);
            }

            if start.elapsed() > duration {
                return Err(nvim_oxi::Error::Api(nvim_oxi::api::Error::Other(
                    "Timed out waiting for Autocmd".into(),
                )));
            }
            nvim_oxi::api::command("sleep 100m")?;
        }
    })
}
