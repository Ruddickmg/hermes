use nvim_oxi::{
    Object,
    conversion::{Error, FromObject},
};

use super::dict_from_object;

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct SessionConfig {
    pub store_history: bool,
}

impl Default for SessionConfig {
    fn default() -> Self {
        Self {
            store_history: true,
        }
    }
}

/// Partial session configuration where each field is optional
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct SessionConfigPartial {
    pub store_history: Option<bool>,
}

impl SessionConfigPartial {
    /// Apply only Some() values to existing config
    pub fn apply_to(self, config: &mut SessionConfig) {
        if let Some(val) = self.store_history {
            config.store_history = val;
        }
    }
}

impl FromObject for SessionConfigPartial {
    fn from_object(obj: Object) -> Result<Self, Error> {
        let dict = dict_from_object(obj)?;
        let store_history = dict
            .get("store_history")
            .map(|o| bool::from_object(o.clone()))
            .transpose()?;
        Ok(Self { store_history })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_session_partial_apply_to_updates_specified() {
        let mut config = SessionConfig::default(); // store_history=true
        let partial = SessionConfigPartial {
            store_history: Some(false),
        };
        partial.apply_to(&mut config);
        assert!(!config.store_history); // changed
    }

    #[test]
    fn test_session_partial_apply_to_preserves_when_none() {
        let mut config = SessionConfig {
            store_history: false,
        };
        let partial = SessionConfigPartial::default(); // None
        partial.apply_to(&mut config);
        assert!(!config.store_history); // preserved
    }

    #[test]
    fn test_session_partial_from_object_parses_correctly() {
        let mut dict = nvim_oxi::Dictionary::new();
        dict.insert("store_history", false);

        let obj = nvim_oxi::Object::from(dict);
        let partial = SessionConfigPartial::from_object(obj).expect("Should parse");

        assert_eq!(partial.store_history, Some(false));
    }

    #[test]
    fn test_session_partial_from_object_empty_dict() {
        let dict = nvim_oxi::Dictionary::default();
        let obj = nvim_oxi::Object::from(dict);
        let partial = SessionConfigPartial::from_object(obj).expect("Should parse");

        assert_eq!(partial.store_history, None);
    }
}
