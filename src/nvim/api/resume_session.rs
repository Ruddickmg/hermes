use nvim_oxi::{
    Dictionary, Object,
    conversion::FromObject,
    lua::{Poppable, Pushable},
};
use std::path::PathBuf;
use tracing::error;

use crate::{
    acp::{Result, error::Error},
    api::{Api, mcp_servers::parse_mcp_servers},
    utilities::{self},
};

#[derive(Debug, Clone, Default)]
pub struct ResumeSessionConfig {
    pub cwd: Option<PathBuf>,
    pub mcp_servers: Vec<agent_client_protocol::schema::McpServer>,
}

impl FromObject for ResumeSessionConfig {
    fn from_object(obj: Object) -> std::result::Result<Self, nvim_oxi::conversion::Error> {
        let dict = crate::nvim::configuration::dict_from_object(obj)?;

        let cwd: Option<PathBuf> = dict.get("cwd").and_then(|obj| {
            obj.clone()
                .try_into()
                .ok()
                .map(|s: nvim_oxi::String| PathBuf::from(s.to_string()))
        });

        let mcp_servers: Vec<agent_client_protocol::schema::McpServer> = dict
            .get("mcpServers")
            .and_then(parse_mcp_servers)
            .unwrap_or_default();

        let current_directory = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        let root = utilities::get_project_root(current_directory, vec![".git".to_string()]);

        Ok(Self {
            cwd: Some(cwd.unwrap_or(root)),
            mcp_servers,
        })
    }
}

impl Poppable for ResumeSessionConfig {
    unsafe fn pop(
        lua_state: *mut nvim_oxi::lua::ffi::State,
    ) -> std::result::Result<Self, nvim_oxi::lua::Error> {
        let obj = unsafe { Object::pop(lua_state)? };
        Ok(Self::from_object(obj)
            .inspect_err(|e| {
                error!(
                    "An error occurred parsing session resume arguments, reverting to defaults: {:?}",
                    e
                )
            })
            .unwrap_or_default())
    }
}

impl Pushable for ResumeSessionConfig {
    unsafe fn push(
        self,
        lua_state: *mut nvim_oxi::lua::ffi::State,
    ) -> std::result::Result<i32, nvim_oxi::lua::Error> {
        let mut dict = Dictionary::new();
        if let Some(cwd) = self.cwd {
            dict.insert("cwd", cwd.to_string_lossy().to_string());
        }
        unsafe { Object::from(dict).push(lua_state) }
    }
}

pub type ResumeSessionArgs = (String, Option<ResumeSessionConfig>);

impl Api {
    #[tracing::instrument(level = "trace", skip(self))]
    pub async fn resume_session(
        &self,
        (session_id, maybe_config): ResumeSessionArgs,
    ) -> Result<()> {
        let config = maybe_config.unwrap_or_default();
        let state = self.state.lock().await;
        let root_markers = state.config.root_markers.clone();
        let agent_info = state.agent_info.clone();
        drop(state);

        if !agent_info.can_resume_sessions() {
            return Ok(());
        }

        let request = agent_client_protocol::schema::ResumeSessionRequest::new(
            agent_client_protocol::schema::SessionId::from(session_id),
            config.cwd.unwrap_or_else(|| {
                let current_dir = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
                crate::utilities::get_project_root(current_dir, root_markers)
            }),
        );

        let connection = self
            .connection
            .get_current_connection()
            .await
            .ok_or_else(|| Error::Connection("No connection found".to_string()))?;

        connection.resume_session(request).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    impl ResumeSessionConfig {
        fn default_with_root() -> Self {
            let current_directory = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
            let root = utilities::get_project_root(current_directory, vec![".git".to_string()]);
            Self {
                cwd: Some(root),
                mcp_servers: Vec::new(),
            }
        }
    }

    fn create_test_config(cwd: Option<&str>) -> ResumeSessionConfig {
        if let Some(path) = cwd {
            ResumeSessionConfig {
                cwd: Some(PathBuf::from(path)),
                mcp_servers: Vec::new(),
            }
        } else {
            ResumeSessionConfig::default_with_root()
        }
    }

    #[test]
    fn config_default_has_cwd() {
        let config = ResumeSessionConfig::default_with_root();
        assert!(config.cwd.is_some());
        assert!(config.mcp_servers.is_empty());
    }

    #[test]
    fn config_with_custom_cwd() {
        let config = create_test_config(Some("/test/path"));
        assert_eq!(config.cwd, Some(PathBuf::from("/test/path")));
        assert!(config.mcp_servers.is_empty());
    }

    #[test]
    fn tuple_type_alias_exists() {
        let _: Option<ResumeSessionArgs> = None;
    }

    #[test]
    fn config_with_mcp_servers() {
        let config = ResumeSessionConfig {
            cwd: Some(PathBuf::from("/project")),
            mcp_servers: vec![],
        };
        assert_eq!(config.cwd, Some(PathBuf::from("/project")));
        assert!(config.mcp_servers.is_empty());
    }

    #[test]
    fn config_pushable_without_cwd() {
        let config = ResumeSessionConfig {
            cwd: None,
            mcp_servers: vec![],
        };
        assert!(config.cwd.is_none());
        assert!(config.mcp_servers.is_empty());
    }
}
