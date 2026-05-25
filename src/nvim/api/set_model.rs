use crate::{
    acp::{Result, error::Error},
    api::Api,
};

/// Tuple for two positional arguments: (session_id, model_id)
pub type SetModelArgs = (String, String);

impl Api {
    #[tracing::instrument(level = "trace", skip(self))]
    pub async fn set_model(&self, (session_id, model_id): SetModelArgs) -> Result<()> {
        let state = self.state.lock().await;
        let legacy = state
            .session_info
            .get(&session_id)
            .ok_or_else(|| Error::SessionNotFound(session_id.clone()))?
            .model_is_legacy();
        drop(state);

        let config_type = "model".to_string();

        if let Some(is_legacy) = legacy {
            let connection = self
                .connection
                .get_current_connection()
                .await
                .ok_or_else(|| Error::Connection("No connection found".to_string()))?;

            if is_legacy {
                connection
                    .set_session_model(agent_client_protocol::schema::SetSessionModelRequest::new(
                        session_id, model_id,
                    ))
                    .await?;
            } else {
                connection
                    .set_config_option(
                        agent_client_protocol::schema::SetSessionConfigOptionRequest::new(
                            session_id,
                            config_type.clone(),
                            model_id,
                        ),
                    )
                    .await?;
            }
            Ok(())
        } else {
            Err(Error::Unsupported(config_type))
        }
    }
}
