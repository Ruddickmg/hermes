//! Abstraction traits over Neovim API calls.
//!
//! These traits decouple Hermes business logic from the concrete Neovim binding
//! crate (currently nvim-oxi, eventually mlua). All direct Neovim API calls in
//! utility modules should route through these traits so that:
//!
//! 1. Unit tests can substitute a mock implementation.
//! 2. The backing crate can be swapped without touching business logic.

use nvim_oxi::{Array, Dictionary, Object, api};

/// Errors that can occur when interacting with the Neovim API.
///
/// This is intentionally a thin wrapper so we can eventually unify
/// nvim-oxi and mlua error types behind a single enum.
#[derive(Debug, thiserror::Error)]
pub enum NvimError {
    #[error(transparent)]
    Api(#[from] api::Error),

    #[error(transparent)]
    NvimOxi(#[from] nvim_oxi::Error),

    #[error(transparent)]
    Libuv(#[from] nvim_oxi::libuv::Error),

    #[error("{0}")]
    Other(String),
}

/// Core trait for interacting with the Neovim API.
///
/// Every method mirrors a `nvim_oxi::api::*` free function. Implementations
/// should forward to the real API; test mocks return canned values.
///
/// # Object safety
///
/// All methods take `&self` and return concrete `Result` types so the trait
/// is object-safe and mockable with `mockall` or hand-rolled mocks.
pub trait NvimApi {
    // ── Buffer management ──────────────────────────────────────────────

    /// List all open buffers (`nvim_list_bufs`).
    fn list_bufs(&self) -> Result<Vec<api::Buffer>, NvimError>;

    /// Create a new buffer (`nvim_create_buf`).
    fn create_buf(&self, listed: bool, scratch: bool) -> Result<api::Buffer, NvimError>;

    // ── Command execution ──────────────────────────────────────────────

    /// Execute an Ex command (`nvim_command`).
    fn command(&self, cmd: &str) -> Result<(), NvimError>;

    // ── Function calls ─────────────────────────────────────────────────

    /// Call a global Vim function (`nvim_call_function`).
    fn call_function(&self, name: &str, args: Array) -> Result<Object, NvimError>;

    // ── Notifications ──────────────────────────────────────────────────

    /// Send a notification via `nvim_notify`.
    fn notify(&self, msg: &str, level: u64, opts: &Dictionary) -> Result<(), NvimError>;

    // ── Buffer-scoped operations ───────────────────────────────────────

    /// Set lines on a buffer (`nvim_buf_set_lines`).
    ///
    /// `start` is inclusive, `end` is exclusive (both 0-indexed).
    fn buf_set_lines(
        &self,
        buf: &api::Buffer,
        start: usize,
        end: usize,
        strict: bool,
        lines: Vec<String>,
    ) -> Result<(), NvimError>;

    /// Get lines from a buffer (`nvim_buf_get_lines`).
    ///
    /// `start` is inclusive, `end` is exclusive (both 0-indexed).
    fn buf_get_lines(
        &self,
        buf: &api::Buffer,
        start: usize,
        end: usize,
        strict: bool,
    ) -> Result<Vec<String>, NvimError>;

    /// Get a buffer's name (`nvim_buf_get_name`).
    fn buf_get_name(&self, buf: &api::Buffer) -> Result<String, NvimError>;

    /// Execute a closure with the buffer as the current buffer (`nvim_buf_call`).
    ///
    /// The closure receives `()` and must return `()`. Results should be
    /// captured via `Rc<Cell>` or `Rc<RefCell>` as needed.
    fn buf_exec<F>(&self, buf: &api::Buffer, f: F) -> Result<(), NvimError>
    where
        F: FnOnce(()) + Send + 'static;

    /// Get a buffer option value (`nvim_get_option_value` with `buf` set).
    fn get_buf_option<T: nvim_oxi::conversion::FromObject>(
        &self,
        name: &str,
        buf: &api::Buffer,
    ) -> Result<T, NvimError>;

    /// Set a buffer option value (`nvim_set_option_value` with `buf` set).
    fn set_buf_option<T: nvim_oxi::conversion::ToObject>(
        &self,
        name: &str,
        value: T,
        buf: &api::Buffer,
    ) -> Result<(), NvimError>;

    /// Delete a buffer (`nvim_buf_delete`).
    fn buf_delete(&self, buf: &api::Buffer, opts: &Dictionary) -> Result<(), NvimError>;

    // ── Window-scoped operations ───────────────────────────────────────

    /// Get a window option value (`nvim_get_option_value` with `win` set).
    fn get_win_option<T: nvim_oxi::conversion::FromObject>(
        &self,
        name: &str,
        win: &api::Window,
    ) -> Result<T, NvimError>;

    // ── Scheduling ─────────────────────────────────────────────────────

    /// Schedule a closure on Neovim's main event loop (`nvim_oxi::schedule`).
    ///
    /// The closure receives `()` and must return `()`.
    fn schedule<F>(&self, f: F) -> Result<(), NvimError>
    where
        F: FnOnce(()) + Send + 'static;

    // ── Job control ────────────────────────────────────────────────────

    /// Start a job (`jobstart`) in the context of a buffer.
    fn jobstart(
        &self,
        buf: &api::Buffer,
        commands: Array,
        config: Dictionary,
    ) -> Result<i64, NvimError>;

    /// Stop a job (`jobstop`).
    fn jobstop(&self, id: i64) -> Result<(), NvimError>;
}

// ── AsyncHandle abstraction ────────────────────────────────────────────────

/// Trait for cross-thread signalling into Neovim's main event loop.
///
/// Replaces `nvim_oxi::libuv::AsyncHandle` so the libuv FFI can be swapped
/// out independently of the rest of the API surface.
pub trait NvimAsyncHandle {
    /// Create a new async handle. The callback fires on the main thread
    /// whenever `send()` is called from any thread.
    fn new<F>(cb: F) -> Result<Self, NvimError>
    where
        Self: Sized,
        F: FnMut() + Send + 'static;

    /// Signal the handle, causing the callback to fire on the main thread.
    fn send(&self) -> Result<(), NvimError>;
}
