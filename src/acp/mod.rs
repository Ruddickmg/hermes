pub mod connection;
pub mod error;
pub mod handler;
pub mod registry;
pub mod session_info;

use std::result::Result as StdResult;

pub type Result<T> = StdResult<T, error::Error>;
