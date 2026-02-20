use agent_client_protocol::{AvailableCommandsUpdate, Result};
use nvim_oxi::Dictionary;

pub fn available_commands_event(update: AvailableCommandsUpdate) -> Result<Dictionary> {
    let mut data: nvim_oxi::Dictionary = nvim_oxi::Dictionary::new();
    data.insert(
        "available_commands",
        format!("{:?}", update.available_commands),
    );

    if let Some(meta) = update.meta {
        data.insert("meta", format!("{:?}", meta));
    }

    Ok(data)
}

#[cfg(test)]
mod tests {
    use super::*;
    use agent_client_protocol::AvailableCommand;

    #[nvim_oxi::test]
    fn test_available_commands_event_ok() {
        let cmd = AvailableCommand::new("read_file".to_string(), "Read a file".to_string());
        let update = AvailableCommandsUpdate::new(vec![cmd]);

        let result = available_commands_event(update);
        assert!(result.is_ok());
    }

    #[nvim_oxi::test]
    fn test_available_commands_event_contains_commands() {
        let cmd = AvailableCommand::new("read_file".to_string(), "Read a file".to_string());
        let update = AvailableCommandsUpdate::new(vec![cmd]);

        let result = available_commands_event(update).unwrap();
        assert!(result.get("available_commands").is_some());
    }

    #[nvim_oxi::test]
    fn test_available_commands_event_with_meta() {
        let cmd = AvailableCommand::new("write_file".to_string(), "Write a file".to_string());
        let meta: serde_json::Map<String, serde_json::Value> =
            serde_json::json!({"source": "agent"})
                .as_object()
                .unwrap()
                .clone();
        let update = AvailableCommandsUpdate::new(vec![cmd]).meta(meta);

        let result = available_commands_event(update);
        assert!(result.is_ok());
    }
}
