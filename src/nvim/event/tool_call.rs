use std::fs;

use agent_client_protocol::{Error, Result, ToolCall, ToolCallContent};
use nvim_oxi::Dictionary;

use crate::nvim::event;

pub fn tool_call_event(tool_call: ToolCall) -> Result<Dictionary> {
    let mut data: nvim_oxi::Dictionary = nvim_oxi::Dictionary::new();
    data.insert("id", tool_call.tool_call_id.to_string());
    data.insert("title", tool_call.title);

    data.insert("kind", format!("{:?}", tool_call.kind));
    data.insert("status", format!("{:?}", tool_call.status));
    let tool_call_content = tool_call
        .content
        .iter()
        .map(|c| match c {
            ToolCallContent::Content(tool_call_content) => event::communication::communication(
                tool_call_content.content.clone(),
            )
            .map(|(mut dict, content_type)| {
                dict.insert("type", content_type.to_lowercase());
                dict
            }),
            ToolCallContent::Terminal(terminal) => {
                let mut dict = Dictionary::new();
                dict.insert("id", terminal.terminal_id.to_string());
                dict.insert("type", "terminal");
                Ok(dict)
            }
            ToolCallContent::Diff(diff) => fs::read_to_string(diff.path.clone())
                .map_err(Error::into_internal_error)
                .map(|path| {
                    let mut dict = Dictionary::new();
                    dict.insert("type", "diff");
                    dict.insert("path", path);
                    dict.insert("new_text", diff.new_text.clone());
                    if let Some(old_text) = diff.old_text.clone() {
                        dict.insert("old_text", old_text);
                    }
                    dict
                }),
            _ => Err(Error::method_not_found()),
        })
        .collect::<Result<Vec<Dictionary>>>()?;
    data.insert("content", nvim_oxi::Array::from_iter(tool_call_content));

    let locations_str = format!("{:?}", tool_call.locations);
    data.insert("locations", locations_str);

    if let Some(input) = tool_call.raw_input {
        data.insert("input", input.to_string());
    }
    if let Some(output) = tool_call.raw_output {
        data.insert("output", output.to_string());
    }

    Ok(data)
}

#[cfg(test)]
mod tests {
    use super::*;
    use agent_client_protocol::ToolCallId;

    #[nvim_oxi::test]
    fn test_tool_call_event_ok() {
        let tool_call = ToolCall::new(ToolCallId::new("call_001"), "Reading file".to_string())
            .kind(agent_client_protocol::ToolKind::Read)
            .status(agent_client_protocol::ToolCallStatus::Pending);

        let result = tool_call_event(tool_call);
        assert!(result.is_ok());
    }

    #[nvim_oxi::test]
    fn test_tool_call_event_contains_id() {
        let tool_call = ToolCall::new(ToolCallId::new("call_001"), "Reading file".to_string())
            .kind(agent_client_protocol::ToolKind::Read)
            .status(agent_client_protocol::ToolCallStatus::Pending);

        let result = tool_call_event(tool_call).unwrap();
        assert!(result.get("id").is_some());
    }

    #[nvim_oxi::test]
    fn test_tool_call_event_with_input_output() {
        let tool_call = ToolCall::new(ToolCallId::new("call_002"), "Writing file".to_string())
            .kind(agent_client_protocol::ToolKind::Edit)
            .status(agent_client_protocol::ToolCallStatus::InProgress)
            .raw_input(serde_json::json!({"path": "/test/file.txt"}))
            .raw_output(serde_json::json!({"bytes_written": 100}));

        let result = tool_call_event(tool_call);
        assert!(result.is_ok());
    }
}
