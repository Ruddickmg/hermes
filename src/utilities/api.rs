//! Thread-local accessor for the Neovim API implementation.
//!
//! All Neovim API calls in utility modules should route through [`api()`]
//! instead of calling `nvim_oxi::api::*` directly. This centralises access
//! so that:
//!
//! 1. The backing implementation can be swapped for tests.
//! 2. All API calls go through a single, auditable entry point.

use std::cell::RefCell;

pub use crate::nvim::api::NvimApi;
pub use crate::nvim::api::NvimError;
use crate::nvim::api::nvim_oxi_impl::NvimOxiApi;

thread_local! {
    static API: RefCell<Option<NvimOxiApi>> = const { RefCell::new(None) };
}

/// Initialise the thread-local API handle. Must be called once during plugin
/// startup (inside `nvim_oxi::plugin` entry point) before any utility
/// function uses the API.
pub fn init() {
    API.with(|cell| {
        *cell.borrow_mut() = Some(NvimOxiApi);
    });
}

/// Returns a copy of the global [`NvimOxiApi`] instance.
///
/// # Panics
///
/// Panics if [`init()`] has not been called yet.
pub fn api() -> NvimOxiApi {
    API.with(|cell| {
        cell.borrow()
            .as_ref()
            .expect("NvimApi not initialised — call api::init() first")
            .clone()
    })
}
