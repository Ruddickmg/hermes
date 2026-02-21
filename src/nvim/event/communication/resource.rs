use agent_client_protocol::{Annotations, EmbeddedResource, EmbeddedResourceResource, Result};
use nvim_oxi::Dictionary;

pub fn parse_annotations(annotations: Annotations) -> Dictionary {
    let mut annotations_dict = Dictionary::new();
    if let Some(audience) = annotations.audience {
        let roles: Vec<String> = audience.iter().map(|r| format!("{:?}", r)).collect();
        annotations_dict.insert("audience", nvim_oxi::Array::from_iter(roles));
    }
    if let Some(last_modified) = annotations.last_modified {
        annotations_dict.insert("last_modified", last_modified);
    }
    if let Some(priority) = annotations.priority {
        annotations_dict.insert("priority", priority);
    }
    annotations_dict
}

pub fn resource_event(block: EmbeddedResource) -> Result<(Dictionary, String)> {
    let mut dict: Dictionary = Dictionary::new();

    let resource_dict = match block.resource {
        EmbeddedResourceResource::TextResourceContents(contents) => {
            let mut inner = Dictionary::new();
            inner.insert("text", contents.text);
            inner.insert("uri", contents.uri);
            if let Some(mime_type) = contents.mime_type {
                inner.insert("mime_type", mime_type);
            }
            inner
        }
        EmbeddedResourceResource::BlobResourceContents(contents) => {
            let mut inner = Dictionary::new();
            inner.insert("blob", contents.blob);
            inner.insert("uri", contents.uri);
            if let Some(mime_type) = contents.mime_type {
                inner.insert("mime_type", mime_type);
            }
            inner
        }
        _ => Dictionary::new(),
    };
    dict.insert("resource", resource_dict);

    if let Some(annotations) = block.annotations {
        dict.insert("annotations", parse_annotations(annotations));
    }
    if let Some(meta) = block.meta {
        dict.insert("meta", format!("{:?}", meta));
    }
    Ok((dict, "Resource".to_string()))
}
