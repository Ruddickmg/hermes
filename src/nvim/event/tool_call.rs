use std::fs;

use agent_client_protocol::{Error, Result, ToolCall, ToolCallContent};
use nvim_oxi::Dictionary;

use crate::nvim::event;

pub fn parse_tool_call_content(content: ToolCallContent) -> Result<Dictionary> {
    match content {
        ToolCallContent::Content(container) => event::communication::communication(
            container.content.clone(),
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
    }
}

pub fn tool_call_event(tool_call: ToolCall) -> Result<Dictionary> {
    let mut data: nvim_oxi::Dictionary = nvim_oxi::Dictionary::new();

    let tool_call_content = tool_call
        .content
        .into_iter()
        .map(parse_tool_call_content)
        .collect::<Result<Vec<Dictionary>>>()?;

    data.insert("content", nvim_oxi::Array::from_iter(tool_call_content));
    data.insert("id", tool_call.tool_call_id.to_string());
    data.insert("title", tool_call.title);
    data.insert("kind", format!("{:?}", tool_call.kind));
    data.insert("status", format!("{:?}", tool_call.status));

    let locations = tool_call
        .locations
        .iter()
        .map(|location| {
            fs::read_to_string(location.path.clone())
                .map(|path| {
                    let mut dict = Dictionary::new();
                    dict.insert("path", path);
                    if let Some(line) = location.line {
                        dict.insert("line", line);
                    }
                    dict
                })
                .map_err(Error::into_internal_error)
        })
        .collect::<Result<Vec<Dictionary>>>()?;

    data.insert("locations", nvim_oxi::Array::from_iter(locations));

    if let Some(input) = tool_call.raw_input {
        data.insert("input", input.to_string());
    }
    if let Some(output) = tool_call.raw_output {
        data.insert("output", output.to_string());
    }

    Ok(data)
}
