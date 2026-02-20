use agent_client_protocol::{CurrentModeUpdate, Result};
use nvim_oxi::Dictionary;

pub fn current_mode_event(update: CurrentModeUpdate) -> Result<Dictionary> {
    let mut data: nvim_oxi::Dictionary = nvim_oxi::Dictionary::new();
    data.insert("current_mode_id", update.current_mode_id.to_string());

    if let Some(meta) = update.meta {
        data.insert("meta", format!("{:?}", meta));
    }

    Ok(data)
}

#[cfg(test)]
mod tests {
    use super::*;
    use agent_client_protocol::SessionModeId;

    #[nvim_oxi::test]
    fn test_current_mode_event_ok() {
        let update = CurrentModeUpdate::new(SessionModeId::new("ask"));

        let result = current_mode_event(update);
        assert!(result.is_ok());
    }

    #[nvim_oxi::test]
    fn test_current_mode_event_contains_mode_id() {
        let update = CurrentModeUpdate::new(SessionModeId::new("ask"));

        let result = current_mode_event(update).unwrap();
        assert!(result.get("current_mode_id").is_some());
    }

    #[nvim_oxi::test]
    fn test_current_mode_event_with_meta() {
        let meta: serde_json::Map<String, serde_json::Value> =
            serde_json::json!({"source": "user"})
                .as_object()
                .unwrap()
                .clone();
        let update = CurrentModeUpdate::new(SessionModeId::new("code")).meta(meta);

        let result = current_mode_event(update);
        assert!(result.is_ok());
    }
}
