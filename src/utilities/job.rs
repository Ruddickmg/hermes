use crate::acp::{Result, error::Error};
use nvim_oxi::{Array, Dictionary, Object, api};

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
    buf.call(|_| api::call_function::<(Array, Dictionary), i64>("jobstart", (commands, config)))
        .inspect_err(|e| tracing::error!("jobstart failed: {:?}", e))
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
    api::call_function::<(i64,), ()>("jobstop", (id,)).map_err(|e| Error::Internal(e.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn start_job_in_buffer_signature_compiles() {
        fn assert_compiles(
            _f: impl Fn(&api::Buffer, String, Vec<String>, Dictionary) -> Result<i64>,
        ) {
        }
        assert_compiles(start_job_in_buffer);
    }

    #[test]
    fn stop_job_signature_compiles() {
        fn assert_compiles(_f: impl Fn(i64) -> Result<()>) {}
        assert_compiles(stop_job);
    }
}
