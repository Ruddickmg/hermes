use agent_client_protocol::schema::{
    NewSessionResponse, SessionConfigKind, SessionConfigOptionCategory, SessionConfigSelectOptions,
};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Default, Clone, PartialEq)]
pub struct HermesOption {
    pub value: String,
    pub name: String,
    pub description: Option<String>,
    pub group: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
pub struct Selection {
    pub options: Vec<HermesOption>,
    pub current: HermesOption,
    #[serde(skip)]
    legacy: bool,
}

#[derive(Debug, Default, Clone)]
pub struct SessionDetails {
    pub modes: Option<Selection>,
    pub models: Option<Selection>,
    pub thought_levels: Option<Selection>,
}

impl SessionDetails {
    pub fn new(session: &NewSessionResponse) -> Self {
        Self {
            modes: Self::parse_modes(session),
            models: Self::parse_models(session),
            thought_levels: Self::parse_thought_levels(session),
        }
    }

    pub fn mode_is_legacy(&self) -> Option<bool> {
        self.modes.as_ref().map(|mode| mode.legacy)
    }

    pub fn mode_options(&self) -> Option<&Vec<HermesOption>> {
        self.modes.as_ref().map(|mode| &mode.options)
    }

    pub fn current_mode(&self) -> Option<&HermesOption> {
        self.modes.as_ref().map(|mode| &mode.current)
    }

    pub fn set_current_mode(&mut self, new_current: HermesOption) {
        if let Some(modes) = &mut self.modes {
            modes.current = new_current;
        }
    }

    pub fn get_mode(&self, value: &str) -> Option<&HermesOption> {
        self.modes
            .as_ref()
            .and_then(|mode| mode.options.iter().find(|option| option.value == value))
    }

    pub fn model_is_legacy(&self) -> Option<bool> {
        self.models.as_ref().map(|model| model.legacy)
    }

    pub fn model_options(&self) -> Option<&Vec<HermesOption>> {
        self.models.as_ref().map(|model| &model.options)
    }

    pub fn current_model(&self) -> Option<&HermesOption> {
        self.models.as_ref().map(|model| &model.current)
    }

    pub fn set_current_model(&mut self, new_current: HermesOption) {
        if let Some(models) = &mut self.models {
            models.current = new_current;
        }
    }

    pub fn get_model(&self, value: &str) -> Option<&HermesOption> {
        self.models
            .as_ref()
            .and_then(|model| model.options.iter().find(|option| option.value == value))
    }

    pub fn set_current_thought_level(&mut self, new_current: HermesOption) {
        if let Some(thought_levels) = &mut self.thought_levels {
            thought_levels.current = new_current;
        }
    }

    pub fn get_thought_level(&self, value: &str) -> Option<&HermesOption> {
        self.thought_levels
            .as_ref()
            .and_then(|tl| tl.options.iter().find(|option| option.value == value))
    }

    pub fn thought_level_options(&self) -> Option<&Vec<HermesOption>> {
        self.thought_levels.as_ref().map(|model| &model.options)
    }

    pub fn current_thought_level(&self) -> Option<&HermesOption> {
        self.thought_levels.as_ref().map(|model| &model.current)
    }

    fn parse_options(
        session: &NewSessionResponse,
        category: SessionConfigOptionCategory,
    ) -> Option<Selection> {
        let mut current_option = String::new();
        let options: Vec<HermesOption> = session
            .config_options
            .as_ref()
            .map(|options| {
                options
                    .iter()
                    .filter_map(|opt| {
                        if let SessionConfigKind::Select(select) = &opt.kind
                            && opt.category.as_ref() == Some(&category)
                        {
                            current_option = select.current_value.to_string();
                            match &select.options {
                                SessionConfigSelectOptions::Grouped(groups) => Some(
                                    groups
                                        .iter()
                                        .flat_map(|group| {
                                            group.options.iter().map(move |opt| HermesOption {
                                                value: opt.value.to_string(),
                                                name: opt.name.to_string(),
                                                description: opt.description.clone(),
                                                group: Some(group.name.to_string()),
                                            })
                                        })
                                        .collect::<Vec<HermesOption>>(),
                                ),
                                SessionConfigSelectOptions::Ungrouped(ungrouped) => Some(
                                    ungrouped
                                        .iter()
                                        .map(|opt| HermesOption {
                                            value: opt.value.to_string(),
                                            name: opt.name.to_string(),
                                            description: opt.description.clone(),
                                            group: None,
                                        })
                                        .collect::<Vec<HermesOption>>(),
                                ),
                                _ => None,
                            }
                        } else {
                            None
                        }
                    })
                    .flatten()
                    .collect::<Vec<HermesOption>>()
            })
            .unwrap_or_default();

        if !options.is_empty() {
            let current = options
                .iter()
                .find(|option| option.value == current_option)
                .cloned()
                .unwrap_or_else(|| {
                    tracing::warn!(
                        "Current value '{}' not found in options for category {:?}, defaulting to first option",
                        current_option,
                        category
                    );
                    options[0].clone()
                });

            Some(Selection {
                current,
                options,
                legacy: false,
            })
        } else {
            None
        }
    }

    fn parse_thought_levels(session: &NewSessionResponse) -> Option<Selection> {
        Self::parse_options(session, SessionConfigOptionCategory::ThoughtLevel)
    }

    fn parse_models(session: &NewSessionResponse) -> Option<Selection> {
        Self::parse_options(session, SessionConfigOptionCategory::Model).or_else(|| {
            let mut current: HermesOption = HermesOption::default();
            session.models.as_ref().map(|models| Selection {
                options: models
                    .available_models
                    .iter()
                    .map(|model| {
                        let option = HermesOption {
                            value: model.model_id.to_string(),
                            name: model.name.to_string(),
                            description: model.description.clone(),
                            group: None,
                        };
                        if option.value == models.current_model_id.to_string() {
                            current = option.clone();
                        }
                        option
                    })
                    .collect(),
                current,
                legacy: true,
            })
        })
    }

    fn parse_modes(session: &NewSessionResponse) -> Option<Selection> {
        Self::parse_options(session, SessionConfigOptionCategory::Mode).or_else(|| {
            let mut current = HermesOption::default();
            session.modes.as_ref().map(|modes| Selection {
                options: modes
                    .available_modes
                    .iter()
                    .map(|mode| {
                        let option = HermesOption {
                            value: mode.id.to_string(),
                            name: mode.name.to_string(),
                            description: mode.description.clone(),
                            group: None,
                        };
                        if option.value == modes.current_mode_id.to_string() {
                            current = option.clone();
                        }
                        option
                    })
                    .collect(),
                current,
                legacy: true,
            })
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use agent_client_protocol::schema::{
        ModelInfo, NewSessionResponse, SessionConfigOption, SessionConfigOptionCategory,
        SessionConfigSelectGroup, SessionConfigSelectOption, SessionMode, SessionModeState,
        SessionModelState,
    };
    use pretty_assertions::assert_eq;

    #[test]
    fn parse_modes_new_config_path_returns_non_legacy() {
        let option = SessionConfigOption::select(
            "mode",
            "Mode",
            "chat",
            vec![
                SessionConfigSelectOption::new("chat", "Chat"),
                SessionConfigSelectOption::new("code", "Code"),
            ],
        )
        .category(SessionConfigOptionCategory::Mode);

        let session = NewSessionResponse::new("test-session").config_options(vec![option]);

        let details = SessionDetails::new(&session);
        assert_eq!(details.mode_is_legacy(), Some(false));
    }

    #[test]
    fn parse_modes_new_config_path_stores_options() {
        let option = SessionConfigOption::select(
            "mode",
            "Mode",
            "chat",
            vec![
                SessionConfigSelectOption::new("chat", "Chat"),
                SessionConfigSelectOption::new("code", "Code"),
            ],
        )
        .category(SessionConfigOptionCategory::Mode);

        let session = NewSessionResponse::new("test-session").config_options(vec![option]);

        let details = SessionDetails::new(&session);
        let options = details.mode_options().unwrap();
        assert_eq!(options.len(), 2);
        assert_eq!(options[0].value, "chat");
        assert_eq!(options[0].name, "Chat");
        assert_eq!(options[0].group, None);
        assert_eq!(options[1].value, "code");
        assert_eq!(options[1].name, "Code");
        assert_eq!(options[1].group, None);
    }

    #[test]
    fn parse_modes_new_config_path_stores_current() {
        let option = SessionConfigOption::select(
            "mode",
            "Mode",
            "chat",
            vec![SessionConfigSelectOption::new("chat", "Chat")],
        )
        .category(SessionConfigOptionCategory::Mode);

        let session = NewSessionResponse::new("test-session").config_options(vec![option]);

        let details = SessionDetails::new(&session);
        assert_eq!(
            details.current_mode().map(|m| m.value.as_str()),
            Some("chat")
        );
    }

    #[test]
    fn parse_modes_legacy_fallback_returns_legacy() {
        let mode = SessionMode::new("chat", "Chat");
        let modes = SessionModeState::new("chat", vec![mode]);

        let session = NewSessionResponse::new("test-session").modes(modes);

        let details = SessionDetails::new(&session);
        assert_eq!(details.mode_is_legacy(), Some(true));
    }

    #[test]
    fn parse_modes_legacy_stores_options() {
        let mode = SessionMode::new("chat", "Chat");
        let modes = SessionModeState::new("chat", vec![mode]);

        let session = NewSessionResponse::new("test-session").modes(modes);

        let details = SessionDetails::new(&session);
        let options = details.mode_options().unwrap();
        assert_eq!(options.len(), 1);
        assert_eq!(options[0].value, "chat");
        assert_eq!(options[0].name, "Chat");
        assert_eq!(options[0].group, None);
    }

    #[test]
    fn parse_modes_neither_present_returns_none() {
        let session = NewSessionResponse::new("test-session");
        let details = SessionDetails::new(&session);
        assert_eq!(details.mode_is_legacy(), None);
        assert_eq!(details.mode_options(), None);
        assert_eq!(details.current_mode(), None);
    }

    #[test]
    fn parse_modes_new_takes_precedence_over_legacy() {
        let option = SessionConfigOption::select(
            "mode",
            "Mode",
            "chat",
            vec![SessionConfigSelectOption::new("chat", "Chat")],
        )
        .category(SessionConfigOptionCategory::Mode);

        let mode = SessionMode::new("chat", "Chat");
        let modes = SessionModeState::new("chat", vec![mode]);

        let session = NewSessionResponse::new("test-session")
            .config_options(vec![option])
            .modes(modes);

        let details = SessionDetails::new(&session);
        assert_eq!(details.mode_is_legacy(), Some(false));
    }

    #[test]
    fn parse_modes_wrong_category_falls_back_to_legacy() {
        let option = SessionConfigOption::select(
            "model",
            "Model",
            "gpt4",
            vec![SessionConfigSelectOption::new("gpt4", "GPT-4")],
        )
        .category(SessionConfigOptionCategory::Model);

        let mode = SessionMode::new("chat", "Chat");
        let modes = SessionModeState::new("chat", vec![mode]);

        let session = NewSessionResponse::new("test-session")
            .config_options(vec![option])
            .modes(modes);

        let details = SessionDetails::new(&session);
        assert_eq!(details.mode_is_legacy(), Some(true));
    }

    #[test]
    fn parse_modes_grouped_flattens_all_groups() {
        let group1 = SessionConfigSelectGroup::new(
            "group1",
            "Group One",
            vec![
                SessionConfigSelectOption::new("chat", "Chat"),
                SessionConfigSelectOption::new("code", "Code"),
            ],
        );
        let group2 = SessionConfigSelectGroup::new(
            "group2",
            "Group Two",
            vec![SessionConfigSelectOption::new("agent", "Agent")],
        );
        let option = SessionConfigOption::new(
            "mode",
            "Mode",
            agent_client_protocol::schema::SessionConfigKind::Select(
                agent_client_protocol::schema::SessionConfigSelect::new(
                    "chat",
                    vec![group1, group2],
                ),
            ),
        )
        .category(SessionConfigOptionCategory::Mode);

        let session = NewSessionResponse::new("test-session").config_options(vec![option]);

        let details = SessionDetails::new(&session);
        let options = details.mode_options().unwrap();
        assert_eq!(options.len(), 3);
    }

    #[test]
    fn parse_modes_grouped_includes_group_name() {
        let group = SessionConfigSelectGroup::new(
            "my-group",
            "My Group",
            vec![SessionConfigSelectOption::new("chat", "Chat")],
        );
        let option = SessionConfigOption::new(
            "mode",
            "Mode",
            agent_client_protocol::schema::SessionConfigKind::Select(
                agent_client_protocol::schema::SessionConfigSelect::new("chat", vec![group]),
            ),
        )
        .category(SessionConfigOptionCategory::Mode);

        let session = NewSessionResponse::new("test-session").config_options(vec![option]);

        let details = SessionDetails::new(&session);
        let options = details.mode_options().unwrap();
        assert_eq!(options.len(), 1);
        assert_eq!(options[0].value, "chat");
        assert_eq!(options[0].name, "Chat");
        assert_eq!(options[0].group, Some("My Group".to_string()));
    }

    #[test]
    fn parse_modes_grouped_current_value_preserved() {
        let group = SessionConfigSelectGroup::new(
            "g",
            "G",
            vec![SessionConfigSelectOption::new("code", "Code")],
        );
        let option = SessionConfigOption::new(
            "mode",
            "Mode",
            agent_client_protocol::schema::SessionConfigKind::Select(
                agent_client_protocol::schema::SessionConfigSelect::new("code", vec![group]),
            ),
        )
        .category(SessionConfigOptionCategory::Mode);

        let session = NewSessionResponse::new("test-session").config_options(vec![option]);

        let details = SessionDetails::new(&session);
        assert_eq!(
            details.current_mode().map(|m| m.value.as_str()),
            Some("code")
        );
        assert_eq!(details.mode_is_legacy(), Some(false));
    }

    #[test]
    fn parse_modes_ungrouped_has_no_group() {
        let option = SessionConfigOption::select(
            "mode",
            "Mode",
            "chat",
            vec![SessionConfigSelectOption::new("chat", "Chat")],
        )
        .category(SessionConfigOptionCategory::Mode);

        let session = NewSessionResponse::new("test-session").config_options(vec![option]);

        let details = SessionDetails::new(&session);
        let options = details.mode_options().unwrap();
        assert_eq!(options[0].group, None);
    }

    #[test]
    fn parse_modes_legacy_has_no_group() {
        let mode = SessionMode::new("chat", "Chat");
        let modes = SessionModeState::new("chat", vec![mode]);

        let session = NewSessionResponse::new("test-session").modes(modes);

        let details = SessionDetails::new(&session);
        let options = details.mode_options().unwrap();
        assert_eq!(options[0].group, None);
    }

    #[test]
    fn parse_modes_grouped_description_preserved() {
        let group = SessionConfigSelectGroup::new(
            "g",
            "G",
            vec![SessionConfigSelectOption::new("chat", "Chat").description("Chat mode")],
        );
        let option = SessionConfigOption::new(
            "mode",
            "Mode",
            agent_client_protocol::schema::SessionConfigKind::Select(
                agent_client_protocol::schema::SessionConfigSelect::new("chat", vec![group]),
            ),
        )
        .category(SessionConfigOptionCategory::Mode);

        let session = NewSessionResponse::new("test-session").config_options(vec![option]);

        let details = SessionDetails::new(&session);
        let options = details.mode_options().unwrap();
        assert_eq!(options[0].description, Some("Chat mode".to_string()));
    }

    #[test]
    fn parse_models_new_config_path_returns_non_legacy() {
        let option = SessionConfigOption::select(
            "model",
            "Model",
            "gpt4",
            vec![SessionConfigSelectOption::new("gpt4", "GPT-4")],
        )
        .category(SessionConfigOptionCategory::Model);

        let session = NewSessionResponse::new("test-session").config_options(vec![option]);

        let details = SessionDetails::new(&session);
        assert_eq!(details.model_is_legacy(), Some(false));
    }

    #[test]
    fn parse_models_legacy_fallback_returns_legacy() {
        let model = ModelInfo::new("gpt4", "GPT-4");
        let models = SessionModelState::new("gpt4", vec![model]);

        let session = NewSessionResponse::new("test-session").models(models);

        let details = SessionDetails::new(&session);
        assert_eq!(details.model_is_legacy(), Some(true));
    }

    #[test]
    fn parse_models_neither_present_returns_none() {
        let session = NewSessionResponse::new("test-session");
        let details = SessionDetails::new(&session);
        assert!(details.model_is_legacy().is_none());
    }
}
