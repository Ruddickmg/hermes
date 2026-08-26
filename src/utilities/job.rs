use crate::acp::{Result, error::Error};
use nvim_oxi::{Array, Dictionary, Object, api};

use super::api::{NvimApi, api};

/// Start a job in the context of a buffer (typically a terminal buffer).
///
/// Calls `jobstart` via `nvim_buf_call` to ensure the job is associated with
/// the given buffer. The command and args are combined into a single array.
///
/// # Errors
///
/// Returns an error if the buffer call or jobstart fails.
#[tracing::instrument(level = "trace", skip(config))]
pub fn start_job_in_buffer(
    buf: &api::Buffer,
    command: String,
    args: Vec<String>,
    config: Dictionary,
) -> Result<i64> {
    let commands: Array = Array::from_iter(vec![command].into_iter().chain(args).map(Object::from));

    // Use Rc<Cell> to capture the result across the FFI boundary inside buf_exec.
    let result = std::rc::Rc::new(std::cell::Cell::new(None));
    let result_inside = result.clone();

    api()
        .buf_exec(buf, move |_| {
            let args = Array::from((commands, config));
            let r = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                api().call_function("jobstart", args)
            }));
            result_inside.set(Some(match r {
                Ok(Ok(obj)) => nvim_oxi::conversion::FromObject::from_object(obj)
                    .map_err(|e| nvim_oxi::Error::Api(nvim_oxi::api::Error::Other(e.to_string()))),
                Ok(Err(e)) => Err(nvim_oxi::Error::Api(nvim_oxi::api::Error::Other(
                    e.to_string(),
                ))),
                Err(e) => Err(nvim_oxi::Error::Api(nvim_oxi::api::Error::Other(format!(
                    "jobstart panicked: {:?}",
                    e.downcast_ref::<&str>().unwrap_or(&"unknown panic")
                )))),
            }));
        })
        .inspect_err(|e| tracing::error!("jobstart failed: {:?}", e))
        .map_err(|e| Error::Internal(e.to_string()))?;

    result
        .take()
        .ok_or_else(|| Error::Internal("jobstart did not produce a result".to_string()))?
        .map_err(|e| Error::Internal(e.to_string()))
}

/// Stop a running job by its ID.
///
/// Calls `jobstop` via `call_function`.
///
/// # Errors
///
/// Returns an error if the jobstop call fails.
#[tracing::instrument(level = "trace")]
pub fn stop_job(id: i64) -> Result<()> {
    api()
        .jobstop(id)
        .map_err(|e| Error::Internal(e.to_string()))
}
