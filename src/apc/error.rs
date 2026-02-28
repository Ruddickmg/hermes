use nvim_oxi::lua;

#[derive(Debug, Clone)]
pub enum Error {
    Internal(String),
    Connection(String),
    Permissions(String),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::Connection(msg) => write!(f, "Connection error: {}", msg),
            Error::Permissions(msg) => write!(f, "Permissions error: {}", msg),
            Error::Internal(msg) => write!(f, "Internal error: {}", msg),
        }
    }
}

impl std::error::Error for Error {}

impl From<Error> for lua::Error {
    fn from(e: Error) -> Self {
        lua::Error::RuntimeError(e.to_string())
    }
}
