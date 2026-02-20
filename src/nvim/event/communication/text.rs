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

#[cfg(test)]
mod tests {
    use super::*;

    #[nvim_oxi::test]
    fn test_text_event_ok() {
        let text = TextContent::new("Hello, world!");
        let result = text_event(text);
        assert!(result.is_ok());
    }

    #[nvim_oxi::test]
    fn test_text_event_contains_text() {
        let text = TextContent::new("Hello, world!");
        let result = text_event(text).unwrap();
        assert!(result.get("text").is_some());
    }

    #[nvim_oxi::test]
    fn test_text_event_with_meta() {
        let meta: serde_json::Map<String, serde_json::Value> =
            serde_json::json!({"source": "test"})
                .as_object()
                .unwrap()
                .clone();
        let text = TextContent::new("Hello, world!").meta(meta);
        let result = text_event(text).unwrap();
        assert!(result.get("meta").is_some());
    }
}
