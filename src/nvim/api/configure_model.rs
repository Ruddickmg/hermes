use nvim_oxi::{
    Dictionary, Object,
    conversion::{Error as ConversionError, FromObject},
    lua::Poppable,
};
use tracing::error;

use crate::{
    acp::{Result, error::Error as AcpError},
    api::Api,
};

/// Tuple for two positional arguments: (session_id, config)
pub type ConfigureModelArgs = (String, ConfigureModelConfig);

/// Table with `id` and `value` keys for configuring a model option.
#[derive(Debug, Clone, Default)]
pub struct ConfigureModelConfig {
    pub id: String,
    pub value: String,
}

impl FromObject for ConfigureModelConfig {
    fn from_object(obj: Object) -> std::result::Result<Self, ConversionError> {
        let dict: Dictionary = obj.try_into()?;

        let id: String = dict
            .get("id")
            .and_then(|o| o.clone().try_into().ok())
            .map(|s: nvim_oxi::String| s.to_string())
            .ok_or_else(|| {
                ConversionError::Other("Missing or invalid 'id' field in config table".to_string())
            })?;

        let value: String = dict
            .get("value")
            .and_then(|o| o.clone().try_into().ok())
            .map(|s: nvim_oxi::String| s.to_string())
            .ok_or_else(|| {
                ConversionError::Other(
                    "Missing or invalid 'value' field in config table".to_string(),
                )
            })?;

        Ok(Self { id, value })
    }
}

impl Poppable for ConfigureModelConfig {
    unsafe fn pop(
        lua_state: *mut nvim_oxi::lua::ffi::State,
    ) -> std::result::Result<Self, nvim_oxi::lua::Error> {
        let obj = unsafe { Object::pop(lua_state)? };
        Ok(Self::from_object(obj)
            .inspect_err(|e| error!("{:?}", e))
            .unwrap_or_default())
    }
}

impl Api {
    #[tracing::instrument(level = "trace", skip(self))]
    pub async fn configure_model(&self, (session_id, config): ConfigureModelArgs) -> Result<()> {
        let state = self.state.lock().await;
        let has_config = state
            .session_info
            .get(&session_id)
            .ok_or_else(|| AcpError::SessionNotFound(session_id.clone()))?
            .model_config_options()
            .iter()
            .any(|mc| mc.id == config.id);
        drop(state);

        if !has_config {
            return Err(AcpError::Internal(format!(
                "Model configuration '{}' not found for session: {}",
                config.id, session_id
            )));
        }

        let connection = self
            .connection
            .get_current_connection()
            .await
            .ok_or_else(|| AcpError::Connection("No connection found".to_string()))?;

        connection
            .set_config_option(
                agent_client_protocol::schema::v1::SetSessionConfigOptionRequest::new(
                    session_id,
                    config.id,
                    config.value,
                ),
            )
            .await?;

        Ok(())
    }
}
