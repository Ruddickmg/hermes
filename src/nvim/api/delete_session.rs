use agent_client_protocol::schema::{DeleteSessionRequest, SessionId};

use crate::{
    acp::{Result, error::Error},
    api::Api,
};

impl Api {
    #[tracing::instrument(level = "trace", skip(self))]
    pub async fn delete_session(&self, session_id: String) -> Result<()> {
        let state = self.state.lock().await;
        let agent_info = state.agent_info.clone();
        drop(state);

        if !agent_info.can_delete_session() {
            return Ok(());
        }

        let request = DeleteSessionRequest::new(SessionId::from(session_id));

        let connection = self
            .connection
            .get_current_connection()
            .await
            .ok_or_else(|| Error::Connection("No connection found".to_string()))?;

        connection.delete_session(request).await
    }
}
