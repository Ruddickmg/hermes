use agent_client_protocol::schema::{
    NewSessionResponse, SessionConfigKind, SessionConfigOptionCategory, SessionConfigSelectOption,
    SessionConfigSelectOptions,
};

#[derive(Debug, Default, Clone)]
pub struct Selection {
    #[allow(dead_code)]
    options: Vec<SessionConfigSelectOption>,
    #[allow(dead_code)]
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
        self.modes.as_ref().map(|mode| mode.legacy).clone()
    }

    fn parse_models(_session: NewSessionResponse) -> Option<Selection> {
        None
    }

    fn parse_modes(session: NewSessionResponse) -> Option<Selection> {
        let selections = session
            .config_options
            .map(|options| {
                options
                    .into_iter()
                    .filter_map(|opt| {
                        if let SessionConfigKind::Select(select) = opt.kind
                            && opt.category == Some(SessionConfigOptionCategory::Mode)
                        {
                            match select.options {
                                // TODO: I'm not sure what grouped is, there is no documentation on it,
                                // figure this out later: https://agentclientprotocol.com/protocol/session-config-options
                                // SessionConfigSelectOptions::Grouped(group) => group.options,
                                SessionConfigSelectOptions::Ungrouped(options) => Some(Selection {
                                    options,
                                    legacy: false,
                                    current: select.current_value.to_string(),
                                }),
                                _ => None,
                            }
                        } else {
                            None
                        }
                    })
                    .collect::<Vec<Selection>>()
            })
            .unwrap_or_default();

        if let Some(modes) = session.modes
            && selections.is_empty()
        {
            Some(Selection {
                options: modes
                    .available_modes
                    .into_iter()
                    .map(|mode| {
                        SessionConfigSelectOption::new(mode.id.to_string(), mode.name.to_string())
                            .description(mode.description)
                            .into()
                    })
                    .collect(),
                current: modes.current_mode_id.to_string(),
                legacy: true,
            })
        } else if let Some(selection) = selections.first().cloned() {
            Some(selection)
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
        SessionConfigSelectOption, SessionMode, SessionModeState,
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
    fn parse_modes_legacy_fallback_returns_legacy() {
        let mode = SessionMode::new("chat", "Chat");
        let modes = SessionModeState::new("chat", vec![mode]);

        let session = NewSessionResponse::new("test-session").modes(modes);

        let details = SessionDetails::new(session);
        assert_eq!(details.mode_is_legacy(), Some(true));
    }

    #[test]
    fn parse_modes_neither_present_returns_none() {
        let session = NewSessionResponse::new("test-session");
        let details = SessionDetails::new(session);
        assert_eq!(details.mode_is_legacy(), None);
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
}
