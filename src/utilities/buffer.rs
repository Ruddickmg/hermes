use crate::acp::{Result, error::Error};
use nvim_oxi::{Array, Dictionary, Object, api, conversion::FromObject};

/// Create a new hidden buffer suitable for terminal use.
///
/// Calls `nvim_create_buf(false, true)` via the direct API.
///
/// # Errors
///
/// Returns an error if buffer creation fails.
pub fn create_hidden_buffer() -> Result<api::Buffer> {
    api::create_buf(false, true).map_err(|e| Error::Internal(e.to_string()))
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
    let args = Array::from((Object::from(buf.handle()), Object::from(opts)));
    api::call_function::<Array, Object>("nvim_buf_delete", args)
        .map_err(|e| Error::Internal(format!("Failed to delete terminal buffer: {}", e)))?;
    Ok(())
}

/// Get the number of lines in a buffer.
///
/// Calls `nvim_buf_line_count` via `call_function`.
///
/// # Errors
///
/// Returns an error if the line count cannot be retrieved.
pub fn buffer_line_count(buf: &api::Buffer) -> Result<usize> {
    let args = Array::from((Object::from(buf.handle()),));
    api::call_function::<Array, i64>("nvim_buf_line_count", args)
        .map(|n| n as usize)
        .map_err(|e| Error::Internal(e.to_string()))
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
    let args = Array::from((
        Object::from(buf.handle()),
        Object::from(start as i64),
        Object::from(end as i64),
        Object::from(strict_indexing),
    ));
    api::call_function::<Array, Array>("nvim_buf_get_lines", args)
        .map(|arr| {
            arr.into_iter()
                .filter_map(|obj| String::from_object(obj).ok())
                .collect()
        })
        .map_err(|e| Error::Internal(e.to_string()))
}
