pub enum Error {
    Connection(String),
    Perimissions(String),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::Connection(msg) => write!(f, "Connection error: {}", msg),
            Error::Perimissions(msg) => write!(f, "Permissions error: {}", msg),
        }
    }
}
