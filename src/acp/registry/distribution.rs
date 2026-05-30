use serde::{Deserialize, Serialize};
use std::fmt::Display;

#[derive(Deserialize, Serialize, std::hash::Hash, PartialEq, Eq, Debug, Clone, Copy)]
pub enum Distribution {
    Uvx,
    Npx,
    Binary,
    Invalid,
}

impl Display for Distribution {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", format!("{:?}", self).to_ascii_lowercase())
    }
}

impl From<&str> for Distribution {
    fn from(value: &str) -> Self {
        match value.to_ascii_lowercase().as_str() {
            "uvx" => Self::Uvx,
            "npx" => Self::Npx,
            "binary" => Self::Binary,
            _ => Self::Invalid,
        }
    }
}

impl From<String> for Distribution {
    fn from(value: String) -> Self {
        Self::from(value.as_str())
    }
}

impl From<&String> for Distribution {
    fn from(value: &String) -> Self {
        Self::from(value.as_str())
    }
}
