use agent_client_protocol::schema::{
    NewSessionResponse, SessionConfigKind, SessionConfigOptionCategory, SessionConfigSelectOption,
    SessionConfigSelectOptions,
};

#[derive(Debug, Default, Clone)]
pub struct Selection {
    options: Vec<SessionConfigSelectOption>,
    current: String,
    legacy: bool,
}

#[derive(Debug, Default, Clone)]
pub struct SessionDetails {
    modes: Option<Selection>,
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

    fn parse_models(session: NewSessionResponse) -> Option<Selection> {
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
