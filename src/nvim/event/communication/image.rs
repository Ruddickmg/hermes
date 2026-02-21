use crate::nvim::event::annotations::parse_annotations;
use agent_client_protocol::{ImageContent, Result};
use nvim_oxi::Dictionary;

pub fn image_event(image: ImageContent) -> Result<(Dictionary, String)> {
    let mut dict: Dictionary = Dictionary::new();
    dict.insert("data", image.data);
    dict.insert("mimeType", image.mime_type);
    if let Some(uri) = image.uri {
        dict.insert("uri", uri);
    }
    if let Some(annotations) = image.annotations {
        dict.insert("annotations", parse_annotations(annotations));
    }
    if let Some(meta) = image.meta {
        dict.insert("meta", format!("{:?}", meta));
    }
    Ok((dict, "Image".to_string()))
}
