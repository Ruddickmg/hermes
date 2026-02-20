use agent_client_protocol::{ConfigOptionUpdate, Result};
use nvim_oxi::Dictionary;

pub fn config_option_event(update: ConfigOptionUpdate) -> Result<Dictionary> {
    let mut data: nvim_oxi::Dictionary = nvim_oxi::Dictionary::new();
    data.insert("config_options", format!("{:?}", update.config_options));

    if let Some(meta) = update.meta {
        data.insert("meta", format!("{:?}", meta));
    }

    Ok(data)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[nvim_oxi::test]
    fn test_config_option_event_ok() {
        let update = ConfigOptionUpdate::new(vec![]);

        let result = config_option_event(update);
        assert!(result.is_ok());
    }

    #[nvim_oxi::test]
    fn test_config_option_event_contains_config_options() {
        let update = ConfigOptionUpdate::new(vec![]);

        let result = config_option_event(update).unwrap();
        assert!(result.get("config_options").is_some());
    }

    #[nvim_oxi::test]
    fn test_config_option_event_with_meta() {
        let meta: serde_json::Map<String, serde_json::Value> =
            serde_json::json!({"source": "agent"})
                .as_object()
                .unwrap()
                .clone();
        let update = ConfigOptionUpdate::new(vec![]).meta(meta);

        let result = config_option_event(update);
        assert!(result.is_ok());
    }
}
