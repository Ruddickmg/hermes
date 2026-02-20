use agent_client_protocol::{Result, ToolCallUpdate};
use nvim_oxi::Dictionary;

pub fn tool_call_update_event(update: ToolCallUpdate) -> Result<Dictionary> {
    let mut data: nvim_oxi::Dictionary = nvim_oxi::Dictionary::new();
    data.insert("tool_call_id", update.tool_call_id.to_string());
    data.insert("fields", format!("{:?}", update.fields));

    if let Some(meta) = update.meta {
        data.insert("meta", format!("{:?}", meta));
    }

    Ok(data)
}

#[cfg(test)]
mod tests {
    use super::*;
    use agent_client_protocol::{ToolCallId, ToolCallUpdateFields};

    #[nvim_oxi::test]
    fn test_tool_call_update_event_ok() {
        let fields =
            ToolCallUpdateFields::new().status(agent_client_protocol::ToolCallStatus::InProgress);
        let update = ToolCallUpdate::new(ToolCallId::new("call_001"), fields);

        let result = tool_call_update_event(update);
        assert!(result.is_ok());
    }

    #[nvim_oxi::test]
    fn test_tool_call_update_event_contains_tool_call_id() {
        let fields =
            ToolCallUpdateFields::new().status(agent_client_protocol::ToolCallStatus::InProgress);
        let update = ToolCallUpdate::new(ToolCallId::new("call_001"), fields);

        let result = tool_call_update_event(update).unwrap();
        assert!(result.get("tool_call_id").is_some());
    }

    #[nvim_oxi::test]
    fn test_tool_call_update_event_with_meta() {
        let fields = ToolCallUpdateFields::new().title("Updated title".to_string());
        let meta: serde_json::Map<String, serde_json::Value> =
            serde_json::json!({"source": "agent"})
                .as_object()
                .unwrap()
                .clone();
        let update = ToolCallUpdate::new(ToolCallId::new("call_002"), fields).meta(meta);

        let result = tool_call_update_event(update);
        assert!(result.is_ok());
    }
}
