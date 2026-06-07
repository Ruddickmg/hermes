//! Version-independent helpers for getting/setting Neovim buffer and window options.
//!
//! These bypass the `OptionOpts` builder from nvim-oxi, whose struct layout
//! changes between neovim feature versions (field names shift in the hashy_hash
//! mask, causing incorrect mask bits at runtime). Instead we call the underlying
//! Neovim API functions directly via `call_function`.

use nvim_oxi::{Array, Dictionary, Object, api};

/// Get a buffer-scoped option value.
pub fn get_buf_option<T: nvim_oxi::conversion::FromObject>(
    name: &str,
    buf: &api::Buffer,
) -> Result<T, nvim_oxi::api::Error> {
    let mut opts = Dictionary::new();
    opts.insert("buf", Object::from(buf.handle()));
    let args = Array::from((Object::from(name), Object::from(opts)));
    api::call_function::<Array, T>("nvim_get_option_value", args)
}

/// Set a buffer-scoped option value.
pub fn set_buf_option<T: nvim_oxi::conversion::ToObject>(
    name: &str,
    value: T,
    buf: &api::Buffer,
) -> Result<(), nvim_oxi::api::Error> {
    let mut opts = Dictionary::new();
    opts.insert("buf", Object::from(buf.handle()));
    let value_obj = value
        .to_object()
        .map_err(|e| nvim_oxi::api::Error::Other(e.to_string()))?;
    let args = Array::from((Object::from(name), value_obj, Object::from(opts)));
    api::call_function::<Array, Object>("nvim_set_option_value", args)?;
    Ok(())
}

/// Get a buffer's name (file path).
pub fn buf_get_name(buf: &api::Buffer) -> Result<String, nvim_oxi::api::Error> {
    let args = Array::from((Object::from(buf.handle()),));
    api::call_function::<Array, String>("nvim_buf_get_name", args)
}

/// Get a window-scoped option value.
pub fn get_win_option<T: nvim_oxi::conversion::FromObject>(
    name: &str,
    win: &api::Window,
) -> Result<T, nvim_oxi::api::Error> {
    let mut opts = Dictionary::new();
    opts.insert("win", Object::from(win.handle()));
    let args = Array::from((Object::from(name), Object::from(opts)));
    api::call_function::<Array, T>("nvim_get_option_value", args)
}
