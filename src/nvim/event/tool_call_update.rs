use crate::nvim::event::parse_tool_call_content;
use agent_client_protocol::{Result, ToolCallUpdate};
use nvim_oxi::Dictionary;

pub fn tool_call_update_event(update: ToolCallUpdate) -> Result<Dictionary> {
    let mut data: nvim_oxi::Dictionary = nvim_oxi::Dictionary::new();

    data.insert("id", update.tool_call_id.to_string());

    if let Some(content) = update.fields.content {
        data.insert(
            "fields",
            nvim_oxi::Array::from_iter(
                content
                    .into_iter()
                    .map(parse_tool_call_content)
                    .collect::<Result<Vec<Dictionary>>>()?,
            ),
        );
    }
    if let Some(meta) = update.meta {
        data.insert("meta", format!("{:?}", meta));
    }

    Ok(data)
}
