use agent_client_protocol::schema::{
    NewSessionResponse, SessionConfigKind, SessionConfigOptionCategory, SessionConfigSelectOptions,
};

#[derive(Debug, Default, Clone, PartialEq)]
pub struct ModeOption {
    pub value: String,
    pub name: String,
    pub description: Option<String>,
    pub group: Option<String>,
}

#[derive(Debug, Default, Clone)]
pub struct Selection {
    options: Vec<ModeOption>,
    current: String,
    legacy: bool,
}

#[derive(Debug, Default, Clone)]
pub struct SessionDetails {
    modes: Option<Selection>,
    #[allow(dead_code)]
    models: Option<Selection>,
}

impl SessionDetails {
    pub fn new(session: NewSessionResponse) -> Self {
        Self {
            modes: Self::parse_modes(session.clone()),
            models: Self::parse_models(session),
        }
    }

    pub fn mode_is_legacy(&self) -> Option<bool> {
        self.modes.as_ref().map(|mode| mode.legacy)
    }

    pub fn mode_options(&self) -> Option<&Vec<ModeOption>> {
        self.modes.as_ref().map(|mode| &mode.options)
    }

    pub fn mode_current(&self) -> Option<&str> {
        self.modes.as_ref().map(|mode| mode.current.as_str())
    }

    fn parse_models(_session: NewSessionResponse) -> Option<Selection> {
        None
    }

    fn parse_modes(session: NewSessionResponse) -> Option<Selection> {
        let mut current = String::new();
        let options: Vec<ModeOption> = session
            .config_options
            .map(|options| {
                options
                    .into_iter()
                    .filter_map(|opt| {
                        if let SessionConfigKind::Select(select) = opt.kind
                            && opt.category == Some(SessionConfigOptionCategory::Mode)
                        {
                            current = select.current_value.0.as_ref().to_string();
                            match select.options {
                                SessionConfigSelectOptions::Grouped(groups) => Some(
                                    groups
                                        .into_iter()
                                        .flat_map(|group| {
                                            group.options.into_iter().map(move |opt| ModeOption {
                                                value: opt.value.0.as_ref().to_string(),
                                                name: opt.name,
                                                description: opt.description,
                                                group: Some(group.name.to_string()),
                                            })
                                        })
                                        .collect::<Vec<ModeOption>>(),
                                ),
                                SessionConfigSelectOptions::Ungrouped(ungrouped) => Some(
                                    ungrouped
                                        .into_iter()
                                        .map(|opt| ModeOption {
                                            value: opt.value.0.as_ref().to_string(),
                                            name: opt.name,
                                            description: opt.description,
                                            group: None,
                                        })
                                        .collect::<Vec<ModeOption>>(),
                                ),
                                _ => None,
                            }
                        } else {
                            None
                        }
                    })
                    .flatten()
                    .collect::<Vec<ModeOption>>()
            })
            .unwrap_or_default();

        if !options.is_empty() {
            Some(Selection {
                options,
                current,
                legacy: false,
            })
        } else if let Some(modes) = session.modes {
            Some(Selection {
                options: modes
                    .available_modes
                    .into_iter()
                    .map(|mode| ModeOption {
                        value: mode.id.0.as_ref().to_string(),
                        name: mode.name,
                        description: mode.description,
                        group: None,
                    })
                    .collect(),
                current: modes.current_mode_id.0.as_ref().to_string(),
                legacy: true,
            })
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use agent_client_protocol::schema::{
        NewSessionResponse, SessionConfigOption, SessionConfigOptionCategory,
        SessionConfigSelectGroup, SessionConfigSelectOption, SessionMode, SessionModeState,
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

        let details = SessionDetails::new(session);
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

        let details = SessionDetails::new(session);
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

        let details = SessionDetails::new(session);
        assert_eq!(details.mode_current(), Some("chat"));
    }

    #[test]
    fn parse_modes_legacy_fallback_returns_legacy() {
        let mode = SessionMode::new("chat", "Chat");
        let modes = SessionModeState::new("chat", vec![mode]);

        let session = NewSessionResponse::new("test-session").modes(modes);

        let details = SessionDetails::new(session);
        assert_eq!(details.mode_is_legacy(), Some(true));
    }

    #[test]
    fn parse_modes_legacy_stores_options() {
        let mode = SessionMode::new("chat", "Chat");
        let modes = SessionModeState::new("chat", vec![mode]);

        let session = NewSessionResponse::new("test-session").modes(modes);

        let details = SessionDetails::new(session);
        let options = details.mode_options().unwrap();
        assert_eq!(options.len(), 1);
        assert_eq!(options[0].value, "chat");
        assert_eq!(options[0].name, "Chat");
        assert_eq!(options[0].group, None);
    }

    #[test]
    fn parse_modes_neither_present_returns_none() {
        let session = NewSessionResponse::new("test-session");
        let details = SessionDetails::new(session);
        assert_eq!(details.mode_is_legacy(), None);
        assert_eq!(details.mode_options(), None);
        assert_eq!(details.mode_current(), None);
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

        let details = SessionDetails::new(session);
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

        let details = SessionDetails::new(session);
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

        let details = SessionDetails::new(session);
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

        let details = SessionDetails::new(session);
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

        let details = SessionDetails::new(session);
        assert_eq!(details.mode_current(), Some("code"));
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

        let details = SessionDetails::new(session);
        let options = details.mode_options().unwrap();
        assert_eq!(options[0].group, None);
    }

    #[test]
    fn parse_modes_legacy_has_no_group() {
        let mode = SessionMode::new("chat", "Chat");
        let modes = SessionModeState::new("chat", vec![mode]);

        let session = NewSessionResponse::new("test-session").modes(modes);

        let details = SessionDetails::new(session);
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

        let details = SessionDetails::new(session);
        let options = details.mode_options().unwrap();
        assert_eq!(options[0].description, Some("Chat mode".to_string()));
    }
}
