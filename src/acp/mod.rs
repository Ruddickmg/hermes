pub mod session_info;
pub mod connection;
pub mod error;
pub mod handler;

use std::result::Result as StdResult;

pub type Result<T> = StdResult<T, error::Error>;
