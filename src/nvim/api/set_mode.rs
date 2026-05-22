use crate::{
    acp::{Result, error::Error, session_info::SessionDetails},
    api::Api,
};

/// Tuple for two positional arguments: (session_id, mode_id)
pub type SetModeArgs = (String, String);

impl Api {
    #[tracing::instrument(level = "trace", skip(self))]
    pub async fn set_mode(&self, (session_id, mode_id): SetModeArgs) -> Result<()> {
        let state = self.state.lock().await;
        let legacy = state
            .session_info
            .get(&session_id)
            .map(|info: &SessionDetails| info.mode_is_legacy())
            .unwrap_or_default();
        drop(state);

        if let Some(is_legacy) = legacy {
            let connection = self
                .connection
                .get_current_connection()
                .await
                .ok_or_else(|| Error::Connection("No connection found".to_string()))?;

            if is_legacy {
                connection
                    .set_mode(agent_client_protocol::schema::SetSessionModeRequest::new(
                        session_id, mode_id,
                    ))
                    .await?;
            } else {
                connection
                    .set_config_option(
                        agent_client_protocol::schema::SetSessionConfigOptionRequest::new(
                            session_id,
                            "mode".to_string(),
                            mode_id,
                        ),
                    )
                    .await?;
            }
        }
        Ok(())
    }
}
