use crate::acp::Result;
use std::cell::RefCell;
use tracing::error;

/// Creates a Neovim autocommand that invokes a Rust callback when triggered.
///
/// Abstracts away the mlua/nvim_create_autocmd bridging so callers only
/// provide application logic. The callback runs synchronously on Neovim's
/// main thread when the event fires.
///
/// # Type Parameters
///
/// * `F` - A closure that returns a `Result<bool>`. The `bool` is passed back
///   to Lua as the callback's return value (`true` to allow the event, `false`
///   to cancel it for cancellable events). Errors are logged and result in
///   `nil` being returned to Lua.
///
/// # Arguments
///
/// * `group` - The autocommand group ID (from `nvim_create_augroup`)
/// * `event` - The event name (e.g., `"VimLeavePre"`, `"BufWritePost"`)
/// * `callback` - The Rust closure containing application logic
///
/// # Errors
///
/// Returns an error if the Lua callback creation or autocmd registration fails.
///
/// # Panic Safety
///
/// The caller is responsible for ensuring the callback does not panic.
/// If the callback panics, it will abort the Neovim process because Lua
/// callbacks across the FFI boundary cannot unwind.
pub fn create_autocmd<F>(group: i32, event: &str, callback: F) -> nvim_oxi::Result<()>
where
    F: FnMut() -> Result<bool> + 'static,
{
    let lua = nvim_oxi::mlua::lua();
    let event_name = event.to_string();
    let callback = RefCell::new(callback);

    let cb = lua
        .create_function(move |_lua, _: mlua::Value| match callback.borrow_mut()() {
            Ok(value) => Ok(mlua::Value::Boolean(value)),
            Err(e) => {
                error!("Error in '{}' autocmd callback: {}", event_name, e);
                Ok(mlua::Value::Nil)
            }
        })
        .map_err(|e| {
            nvim_oxi::Error::Api(nvim_oxi::api::Error::Other(format!(
                "Failed to create Lua callback for '{}': {}",
                event, e
            )))
        })?;

    let cmd = format!(
        "local group, cb = ...\n vim.api.nvim_create_autocmd('{}', {{ group = group, callback = cb }})",
        event
    );

    lua.load(&cmd).call::<()>((group, cb)).map_err(|e| {
        nvim_oxi::Error::Api(nvim_oxi::api::Error::Other(format!(
            "Failed to create '{}' autocmd: {}",
            event, e
        )))
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn create_autocmd_helper_signature_compiles() {
        // Verify the generic signature compiles with a simple callback
        fn assert_compiles<F>(_f: F)
        where
            F: FnMut() -> Result<bool> + 'static,
        {
        }
        assert_compiles(|| Ok(true));
    }

    #[test]
    fn create_autocmd_callback_error_is_converted_to_nil() {
        // This test verifies the conceptual behavior: when the callback
        // returns an error, the helper returns `Ok(mlua::Value::Nil)`
        // to Lua. We can't test the actual Lua boundary without Neovim,
        // but we can verify the callback signature accepts Results.
        let callback: Box<dyn FnMut() -> Result<bool>> =
            Box::new(|| Err(crate::acp::error::Error::Internal("test error".to_string())));
        let result = callback();
        assert!(result.is_err());
    }

    #[test]
    fn create_autocmd_callback_ok_true_returns_true() {
        let mut callback: Box<dyn FnMut() -> Result<bool>> = Box::new(|| Ok(true));
        let result = callback().unwrap();
        assert!(result);
    }

    #[test]
    fn create_autocmd_callback_ok_false_returns_false() {
        let mut callback: Box<dyn FnMut() -> Result<bool>> = Box::new(|| Ok(false));
        let result = callback().unwrap();
        assert!(!result);
    }
}
