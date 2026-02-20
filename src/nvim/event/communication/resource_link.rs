use agent_client_protocol::{ResourceLink, Result};
use nvim_oxi::Dictionary;

pub fn resource_link_event(block: ResourceLink) -> Result<(Dictionary, String)> {
    let mut dict: Dictionary = Dictionary::new();
    dict.insert("name", block.name);
    dict.insert("uri", block.uri);
    if let Some(description) = block.description {
        dict.insert("description", description);
    }
    if let Some(mime_type) = block.mime_type {
        dict.insert("mime_type", mime_type);
    }
    if let Some(size) = block.size {
        dict.insert("size", size);
    }
    if let Some(title) = block.title {
        dict.insert("title", title);
    }
    if let Some(annotations) = block.annotations {
        dict.insert("annotations", format!("{:?}", annotations));
    }
    if let Some(meta) = block.meta {
        dict.insert("meta", format!("{:?}", meta));
    }
    Ok((dict, "ResourceLink".to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[nvim_oxi::test]
    fn test_resource_link_event_ok() {
        let block = ResourceLink::new("test.txt", "file:///test.txt");
        let result = resource_link_event(block);
        assert!(result.is_ok());
    }

    #[nvim_oxi::test]
    fn test_resource_link_event_contains_name() {
        let block = ResourceLink::new("test.txt", "file:///test.txt");
        let result = resource_link_event(block).unwrap();
        assert!(result.get("name").is_some());
    }

    #[nvim_oxi::test]
    fn test_resource_link_event_contains_uri() {
        let block = ResourceLink::new("test.txt", "file:///test.txt");
        let result = resource_link_event(block).unwrap();
        assert!(result.get("uri").is_some());
    }

    #[nvim_oxi::test]
    fn test_resource_link_event_with_description() {
        let block = ResourceLink::new("test.txt", "file:///test.txt")
            .description("A test file".to_string());
        let result = resource_link_event(block).unwrap();
        assert!(result.get("description").is_some());
    }

    #[nvim_oxi::test]
    fn test_resource_link_event_with_mime_type() {
        let block =
            ResourceLink::new("test.txt", "file:///test.txt").mime_type("text/plain".to_string());
        let result = resource_link_event(block).unwrap();
        assert!(result.get("mime_type").is_some());
    }

    #[nvim_oxi::test]
    fn test_resource_link_event_with_size() {
        let block = ResourceLink::new("test.txt", "file:///test.txt").size(100);
        let result = resource_link_event(block).unwrap();
        assert!(result.get("size").is_some());
    }

    #[nvim_oxi::test]
    fn test_resource_link_event_with_title() {
        let block =
            ResourceLink::new("test.txt", "file:///test.txt").title("Test File".to_string());
        let result = resource_link_event(block).unwrap();
        assert!(result.get("title").is_some());
    }

    #[nvim_oxi::test]
    fn test_resource_link_event_with_meta() {
        let meta: serde_json::Map<String, serde_json::Value> =
            serde_json::json!({"source": "test"})
                .as_object()
                .unwrap()
                .clone();
        let block = ResourceLink::new("test.txt", "file:///test.txt").meta(meta);
        let result = resource_link_event(block).unwrap();
        assert!(result.get("meta").is_some());
    }
}
