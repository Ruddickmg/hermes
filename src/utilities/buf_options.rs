//! Version-independent helpers for getting/setting Neovim buffer and window options.
//!
//! These bypass the `OptionOpts` builder from nvim-oxi, whose struct layout
//! changes between neovim feature versions (field names shift in the hashy_hash
//! mask, causing incorrect mask bits at runtime). Instead we call the underlying
//! Neovim API functions directly via `call_function`.

use nvim_oxi::{api, conversion::FromObject};

use super::api::{NvimApi, api};

/// Get a buffer-scoped option value.
pub fn get_buf_option<T: FromObject>(
    name: &str,
    buf: &api::Buffer,
) -> Result<T, nvim_oxi::api::Error> {
    api()
        .get_buf_option(name, buf)
        .map_err(|e| nvim_oxi::api::Error::Other(e.to_string()))
}

/// Set a buffer-scoped option value.
pub fn set_buf_option<T: nvim_oxi::conversion::ToObject>(
    name: &str,
    value: T,
    buf: &api::Buffer,
) -> Result<(), nvim_oxi::api::Error> {
    api()
        .set_buf_option(name, value, buf)
        .map_err(|e| nvim_oxi::api::Error::Other(e.to_string()))
}

/// Get a buffer's name (file path).
pub fn buf_get_name(buf: &api::Buffer) -> Result<String, nvim_oxi::api::Error> {
    api()
        .buf_get_name(buf)
        .map_err(|e| nvim_oxi::api::Error::Other(e.to_string()))
}

/// Get a window-scoped option value.
pub fn get_win_option<T: FromObject>(
    name: &str,
    win: &api::Window,
) -> Result<T, nvim_oxi::api::Error> {
    api()
        .get_win_option(name, win)
        .map_err(|e| nvim_oxi::api::Error::Other(e.to_string()))
}
