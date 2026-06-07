//! UI waiting utilities for integration tests
//! Pattern based on e2e/src/utilities/autocommand.rs
use hermes::acp::error::Error;
use hermes::utilities::buf_options::get_win_option;
use std::time::{Duration, Instant};
use tracing::debug;

/// Poll with sleep until condition is met or timeout
pub fn wait_for<F>(condition: F, timeout: Duration) -> bool
where
    F: Fn() -> bool,
{
    wait_for_some(timeout, || if condition() { Some(()) } else { None }).is_ok()
}

pub fn wait_for_some<F, R>(timeout: Duration, callback: F) -> Result<R, Error>
where
    F: Fn() -> Option<R>,
{
    let start = Instant::now();
    loop {
        let result = callback();
        if let Some(value) = result {
            return Ok(value);
        }
        if start.elapsed() > timeout {
            return Err(Error::Internal(format!(
                "Timeout after {:?} waiting for condition",
                timeout
            )));
        }
        nvim_oxi::api::command("sleep 10m").ok();
    }
}

/// Check if a floating window exists
/// Floating windows have a non-empty 'relative' field in their config
pub fn find_floating_window() -> Option<nvim_oxi::api::Window> {
    let wins = nvim_oxi::api::list_wins();
    debug!("Total windows: {}", wins.len());

    wins.into_iter().find(|win| {
        // Floating windows have a 'relative' option set (to 'editor', 'win', or 'cursor')
        let is_floating = get_win_option::<String>("relative", win)
            .map(|rel| {
                let floating = !rel.is_empty();
                debug!("Window relative='{}', floating={}", rel, floating);
                floating
            })
            .unwrap_or(false);

        debug!("Window: {:#?}, is_floating: {}", win, is_floating);

        is_floating
    })
}

/// Wait for floating window to appear
pub fn wait_for_floating_window(timeout: Duration) -> Option<nvim_oxi::api::Window> {
    wait_for_some(timeout, find_floating_window).ok()
}

/// Wait for channel to receive outcome
pub fn wait_for_outcome<T>(
    receiver: &mut async_channel::Receiver<T>,
    timeout: Duration,
) -> Option<T> {
    let start = Instant::now();
    loop {
        match receiver.try_recv() {
            Ok(outcome) => return Some(outcome),
            Err(_) => {
                if start.elapsed() > timeout {
                    return None;
                }
                nvim_oxi::api::command("sleep 10m").ok();
            }
        }
    }
}
