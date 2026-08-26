use crate::acp::{Result, error::Error};
use nvim_oxi::{Dictionary, Object, api};

use super::api::{NvimApi, api};

/// Create a new hidden buffer suitable for terminal use.
///
/// Calls `nvim_create_buf(false, true)` via the direct API.
///
/// # Errors
///
/// Returns an error if buffer creation fails.
pub fn create_hidden_buffer() -> Result<api::Buffer> {
    api()
        .create_buf(false, true)
        .map_err(|e| Error::Internal(e.to_string()))
}

/// Delete a buffer, forcing the operation.
///
/// Bypasses the `BufDeleteOpts` builder by calling `nvim_buf_delete`
/// directly via `call_function` with a constructed Dictionary.
///
/// # Errors
///
/// Returns an error if the buffer deletion fails.
pub fn delete_buffer_force(buf: &api::Buffer) -> Result<()> {
    let mut opts = Dictionary::new();
    opts.insert("force", Object::from(true));
    api()
        .buf_delete(buf, &opts)
        .map_err(|e| Error::Internal(format!("Failed to delete terminal buffer: {}", e)))
}

/// Get the number of lines in a buffer.
///
/// Calls `nvim_buf_line_count` via `call_function`.
///
/// # Errors
///
/// Returns an error if the line count cannot be retrieved.
pub fn buffer_line_count(buf: &api::Buffer) -> Result<usize> {
    let args = nvim_oxi::Array::from((Object::from(buf.handle()),));
    let obj = api()
        .call_function("nvim_buf_line_count", args)
        .map_err(|e| Error::Internal(e.to_string()))?;
    let n: i64 = nvim_oxi::conversion::FromObject::from_object(obj)
        .map_err(|e| Error::Internal(e.to_string()))?;
    Ok(n as usize)
}

/// Get lines from a buffer.
///
/// Calls `nvim_buf_get_lines` via `call_function`.
///
/// # Errors
///
/// Returns an error if the lines cannot be retrieved.
pub fn buffer_get_lines(
    buf: &api::Buffer,
    start: usize,
    end: usize,
    strict_indexing: bool,
) -> Result<Vec<String>> {
    api()
        .buf_get_lines(buf, start, end, strict_indexing)
        .map_err(|e| Error::Internal(e.to_string()))
}
