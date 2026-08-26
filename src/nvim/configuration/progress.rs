use nvim_oxi::{
    Object,
    conversion::{Error, FromObject},
};

use super::dict_from_object;

/// Apply the `progress.cmdline` setting to Neovim's `messagesopt` option.
/// When disabled (default), removes `progress:c` to suppress cmdline progress output.
/// When enabled, ensures `progress:c` is set.
pub fn show_progress_in_cmdline(enabled: bool) {
    if crate::utilities::notification_messenger::messagesopt_exists() {
        if enabled {
            nvim_oxi::api::command("set messagesopt+=progress:c").ok();
        } else {
            nvim_oxi::api::command("set messagesopt-=progress:c").ok();
        }
    }
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct ProgressConfig {
    pub cmdline: bool,
    pub update_frequency: u64,
}

impl Default for ProgressConfig {
    fn default() -> Self {
        Self {
            cmdline: false,
            update_frequency: 150,
        }
    }
}

/// Partial progress configuration where each field is optional
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct ProgressConfigPartial {
    pub cmdline: Option<bool>,
    pub update_frequency: Option<u64>,
}

impl ProgressConfigPartial {
    /// Apply only Some() values to existing config
    pub fn apply_to(self, config: &mut ProgressConfig) {
        if let Some(val) = self.cmdline {
            config.cmdline = val;
        }
        if let Some(val) = self.update_frequency {
            config.update_frequency = val;
        }
    }
}

impl FromObject for ProgressConfigPartial {
    fn from_object(obj: Object) -> Result<Self, Error> {
        let dict = dict_from_object(obj)?;
        let cmdline = dict
            .get("cmdline")
            .map(|o| bool::from_object(o.clone()))
            .transpose()?;
        let update_frequency = dict
            .get("update_frequency")
            .map(|o| u64::from_object(o.clone()))
            .transpose()?;
        Ok(Self {
            cmdline,
            update_frequency,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_progress_partial_apply_to_updates_specified() {
        let mut config = ProgressConfig::default();
        let partial = ProgressConfigPartial {
            cmdline: Some(true),
            update_frequency: Some(500),
        };
        partial.apply_to(&mut config);
        assert!(config.cmdline);
        assert_eq!(config.update_frequency, 500);
    }

    #[test]
    fn test_progress_partial_apply_to_preserves_when_none() {
        let mut config = ProgressConfig {
            cmdline: true,
            update_frequency: 300,
        };
        let partial = ProgressConfigPartial::default();
        partial.apply_to(&mut config);
        assert!(config.cmdline);
        assert_eq!(config.update_frequency, 300);
    }

    #[test]
    fn test_progress_partial_from_object_parses_correctly() {
        let mut dict = nvim_oxi::Dictionary::new();
        dict.insert("update_frequency", 500i64);
        dict.insert("cmdline", true);

        let obj = nvim_oxi::Object::from(dict);
        let partial = ProgressConfigPartial::from_object(obj).expect("Should parse");

        assert_eq!(partial.cmdline, Some(true));
        assert_eq!(partial.update_frequency, Some(500));
    }

    #[test]
    fn test_progress_partial_from_object_empty_dict() {
        let dict = nvim_oxi::Dictionary::default();
        let obj = nvim_oxi::Object::from(dict);
        let partial = ProgressConfigPartial::from_object(obj).expect("Should parse");

        assert_eq!(partial.cmdline, None);
        assert_eq!(partial.update_frequency, None);
    }
}
