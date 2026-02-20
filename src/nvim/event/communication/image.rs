use agent_client_protocol::{ImageContent, Result};
use nvim_oxi::Dictionary;

pub fn image_event(image: ImageContent) -> Result<(Dictionary, String)> {
    let mut dict: Dictionary = Dictionary::new();
    dict.insert("data", image.data);
    dict.insert("mime_type", image.mime_type);
    if let Some(uri) = image.uri {
        dict.insert("uri", uri);
    }
    if let Some(annotations) = image.annotations {
        dict.insert("annotations", format!("{:?}", annotations));
    }
    if let Some(meta) = image.meta {
        dict.insert("meta", format!("{:?}", meta));
    }
    Ok((dict, "Image".to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[nvim_oxi::test]
    fn test_image_event_ok() {
        let image = ImageContent::new("base64data", "image/png");
        let result = image_event(image);
        assert!(result.is_ok());
    }

    #[nvim_oxi::test]
    fn test_image_event_contains_data() {
        let image = ImageContent::new("base64data", "image/png");
        let result = image_event(image).unwrap();
        assert!(result.get("data").is_some());
    }

    #[nvim_oxi::test]
    fn test_image_event_contains_mime_type() {
        let image = ImageContent::new("base64data", "image/png");
        let result = image_event(image).unwrap();
        assert!(result.get("mime_type").is_some());
    }

    #[nvim_oxi::test]
    fn test_image_event_with_uri() {
        let image =
            ImageContent::new("base64data", "image/png").uri("file:///image.png".to_string());
        let result = image_event(image).unwrap();
        assert!(result.get("uri").is_some());
    }

    #[nvim_oxi::test]
    fn test_image_event_with_meta() {
        let meta: serde_json::Map<String, serde_json::Value> =
            serde_json::json!({"source": "test"})
                .as_object()
                .unwrap()
                .clone();
        let image = ImageContent::new("base64data", "image/png").meta(meta);
        let result = image_event(image).unwrap();
        assert!(result.get("meta").is_some());
    }
}
