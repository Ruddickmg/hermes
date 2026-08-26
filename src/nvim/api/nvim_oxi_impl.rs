//!nvim-oxi backed implementation of the [`NvimApi`] trait.
//!
//! This is the "real" implementation that delegates to `nvim_oxi::api::*`.
//! It will be replaced once we migrate to mlua.

use nvim_oxi::api::types::LogLevel;
use nvim_oxi::{Array, Dictionary, Object, api};

use super::traits::{NvimApi, NvimAsyncHandle, NvimError};

/// Zero-sized marker type — all state lives in `nvim_oxi` global FFI state.
#[derive(Clone, Copy)]
pub struct NvimOxiApi;

impl NvimApi for NvimOxiApi {
    // ── Buffer management ──────────────────────────────────────────────

    fn list_bufs(&self) -> Result<Vec<api::Buffer>, NvimError> {
        Ok(api::list_bufs().into_iter().collect())
    }

    fn create_buf(&self, listed: bool, scratch: bool) -> Result<api::Buffer, NvimError> {
        Ok(api::create_buf(listed, scratch)?)
    }

    // ── Command execution ──────────────────────────────────────────────

    fn command(&self, cmd: &str) -> Result<(), NvimError> {
        Ok(api::command(cmd)?)
    }

    // ── Function calls ─────────────────────────────────────────────────

    fn call_function(&self, name: &str, args: Array) -> Result<Object, NvimError> {
        Ok(api::call_function(name, args)?)
    }

    // ── Notifications ──────────────────────────────────────────────────

    fn notify(&self, msg: &str, level: u64, opts: &Dictionary) -> Result<(), NvimError> {
        let log_level = match level {
            0 => LogLevel::Trace,
            1 => LogLevel::Debug,
            2 => LogLevel::Info,
            3 => LogLevel::Warn,
            4 => LogLevel::Error,
            _ => LogLevel::Off,
        };
        api::notify(msg, log_level, opts)?;
        Ok(())
    }

    // ── Buffer-scoped operations ───────────────────────────────────────

    fn buf_set_lines(
        &self,
        buf: &api::Buffer,
        start: usize,
        end: usize,
        strict: bool,
        lines: Vec<String>,
    ) -> Result<(), NvimError> {
        let mut buf = buf.clone();
        buf.set_lines(start..end, strict, lines)?;
        Ok(())
    }

    fn buf_get_lines(
        &self,
        buf: &api::Buffer,
        start: usize,
        end: usize,
        strict: bool,
    ) -> Result<Vec<String>, NvimError> {
        let raw = buf.get_lines(start..end, strict)?;
        Ok(raw.map(|s| s.to_string_lossy().into_owned()).collect())
    }

    fn buf_get_name(&self, buf: &api::Buffer) -> Result<String, NvimError> {
        let path = buf.get_name()?;
        Ok(path.to_string_lossy().into_owned())
    }

    fn buf_exec<F>(&self, buf: &api::Buffer, f: F) -> Result<(), NvimError>
    where
        F: FnOnce(()) + 'static,
    {
        buf.call(f)?;
        Ok(())
    }

    fn get_buf_option<T: nvim_oxi::conversion::FromObject>(
        &self,
        name: &str,
        buf: &api::Buffer,
    ) -> Result<T, NvimError> {
        let mut opts = Dictionary::new();
        opts.insert("buf", Object::from(buf.handle()));
        let args = Array::from((Object::from(name), Object::from(opts)));
        Ok(api::call_function("nvim_get_option_value", args)?)
    }

    fn set_buf_option<T: nvim_oxi::conversion::ToObject>(
        &self,
        name: &str,
        value: T,
        buf: &api::Buffer,
    ) -> Result<(), NvimError> {
        let mut opts = Dictionary::new();
        opts.insert("buf", Object::from(buf.handle()));
        let value_obj = value
            .to_object()
            .map_err(|e| NvimError::Other(e.to_string()))?;
        let args = Array::from((Object::from(name), value_obj, Object::from(opts)));
        let _: Object = api::call_function("nvim_set_option_value", args)?;
        Ok(())
    }

    fn buf_delete(&self, buf: &api::Buffer, opts: &Dictionary) -> Result<(), NvimError> {
        let args = Array::from((Object::from(buf.handle()), Object::from(opts.clone())));
        let _: Object = api::call_function("nvim_buf_delete", args)?;
        Ok(())
    }

    // ── Window-scoped operations ───────────────────────────────────────

    fn get_win_option<T: nvim_oxi::conversion::FromObject>(
        &self,
        name: &str,
        win: &api::Window,
    ) -> Result<T, NvimError> {
        let mut opts = Dictionary::new();
        opts.insert("win", Object::from(win.handle()));
        let args = Array::from((Object::from(name), Object::from(opts)));
        Ok(api::call_function("nvim_get_option_value", args)?)
    }

    // ── Scheduling ─────────────────────────────────────────────────────

    fn schedule<F>(&self, f: F) -> Result<(), NvimError>
    where
        F: FnOnce(()) + Send + 'static,
    {
        nvim_oxi::schedule(f);
        Ok(())
    }

    // ── Job control ────────────────────────────────────────────────────

    fn jobstart(
        &self,
        buf: &api::Buffer,
        commands: Array,
        config: Dictionary,
    ) -> Result<i64, NvimError> {
        let result = std::rc::Rc::new(std::cell::Cell::new(None));
        let result_inside = result.clone();

        buf.call(move |_| {
            let r = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                api::call_function::<(Array, Dictionary), i64>("jobstart", (commands, config))
            }));
            result_inside.set(Some(match r {
                Ok(v) => v.map_err(|e| nvim_oxi::Error::Api(e)),
                Err(e) => Err(nvim_oxi::Error::Api(nvim_oxi::api::Error::Other(format!(
                    "jobstart panicked: {:?}",
                    e.downcast_ref::<&str>().unwrap_or(&"unknown panic")
                )))),
            }));
        })?;

        result
            .take()
            .ok_or_else(|| NvimError::Other("jobstart did not produce a result".into()))?
            .map_err(NvimError::NvimOxi)
    }

    fn jobstop(&self, id: i64) -> Result<(), NvimError> {
        let ret: i64 = api::call_function("jobstop", (id,))?;
        if ret == 0 {
            Err(NvimError::Other(format!(
                "jobstop failed: job {} not found",
                id
            )))
        } else {
            Ok(())
        }
    }
}

// ── AsyncHandle ────────────────────────────────────────────────────────────

/// Wraps `nvim_oxi::libuv::AsyncHandle`.
pub struct NvimOxiAsyncHandle {
    inner: nvim_oxi::libuv::AsyncHandle,
}

impl NvimAsyncHandle for NvimOxiAsyncHandle {
    fn new<F>(cb: F) -> Result<Self, NvimError>
    where
        Self: Sized,
        F: FnMut() + Send + 'static,
    {
        let inner = nvim_oxi::libuv::AsyncHandle::new(cb)?;
        Ok(Self { inner })
    }

    fn send(&self) -> Result<(), NvimError> {
        self.inner.send()?;
        Ok(())
    }
}
