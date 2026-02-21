use agent_client_protocol::{Result, TextContent};
use nvim_oxi::Dictionary;

pub fn text_event(text: TextContent) -> Result<(Dictionary, String)> {
    let mut dict: Dictionary = Dictionary::new();
    dict.insert("text", text.text);
    if let Some(annotations) = text.annotations {
        dict.insert("annotations", format!("{:?}", annotations));
    }
    if let Some(meta) = text.meta {
        dict.insert("meta", format!("{:?}", meta));
    }
    Ok((dict, "Text".to_string()))
}
