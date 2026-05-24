use crate::{
    acp::{Result, error::Error},
    api::Api,
};
use nvim_oxi::{Array, Dictionary, Object};

/// Single positional argument: session_id
pub type ThoughtLevelsArgs = String;

impl Api {
    #[tracing::instrument(level = "trace", skip(self))]
    pub async fn thought_levels(&self, session_id: String) -> Result<Array> {
        let state = self.state.lock().await;
        let details = state
            .session_info
            .get(&session_id)
            .ok_or_else(|| Error::SessionNotFound(session_id.clone()))?
            .clone();
        drop(state);

        let options = details
            .thought_level_options()
            .ok_or_else(|| Error::Unsupported("thought_level".to_string()))?;

        Ok(Array::from_iter(options.iter().map(|opt| {
            let mut dict = Dictionary::new();
            dict.insert("value", opt.value.clone());
            dict.insert("name", opt.name.clone());
            if let Some(description) = opt.description.clone() {
                dict.insert("description", description);
            }
            if let Some(group) = opt.group.clone() {
                dict.insert("group", group);
            }
            Object::from(dict)
        })))
    }
}
