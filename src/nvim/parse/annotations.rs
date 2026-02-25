use agent_client_protocol::Annotations;
use nvim_oxi::Dictionary;

pub fn parse_annotations(annotations: Annotations) -> Dictionary {
    let mut annotations_dict = Dictionary::new();
    if let Some(audience) = annotations.audience {
        let roles: Vec<String> = audience.iter().map(|r| format!("{:?}", r)).collect();
        annotations_dict.insert("audience", nvim_oxi::Array::from_iter(roles));
    }
    if let Some(last_modified) = annotations.last_modified {
        annotations_dict.insert("lastModified", last_modified);
    }
    if let Some(priority) = annotations.priority {
        annotations_dict.insert("priority", priority);
    }
    annotations_dict
}
