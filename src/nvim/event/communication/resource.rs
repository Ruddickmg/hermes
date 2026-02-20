use agent_client_protocol::{EmbeddedResource, Result};
use nvim_oxi::Dictionary;

pub fn resource_event(block: EmbeddedResource) -> Result<(Dictionary, String)> {
    let mut dict: Dictionary = Dictionary::new();
    dict.insert("resource", format!("{:?}", block.resource));
    if let Some(annotations) = block.annotations {
        dict.insert("annotations", format!("{:?}", annotations));
    }
    if let Some(meta) = block.meta {
        dict.insert("meta", format!("{:?}", meta));
    }
    Ok((dict, "Resource".to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[nvim_oxi::test]
    fn test_resource_event_ok() {
        let resource = agent_client_protocol::EmbeddedResourceResource::TextResourceContents(
            agent_client_protocol::TextResourceContents::new(
                "test.txt".to_string(),
                "Hello world".to_string(),
            ),
        );
        let block = EmbeddedResource::new(resource);
        let result = resource_event(block);
        assert!(result.is_ok());
    }

    #[nvim_oxi::test]
    fn test_resource_event_contains_resource() {
        let resource = agent_client_protocol::EmbeddedResourceResource::TextResourceContents(
            agent_client_protocol::TextResourceContents::new(
                "test.txt".to_string(),
                "Hello world".to_string(),
            ),
        );
        let block = EmbeddedResource::new(resource);
        let result = resource_event(block).unwrap();
        assert!(result.get("resource").is_some());
    }
}
