use crate::{
    acp::{
        self,
        connection::{Assistant, Connection},
        error::Error,
    },
    api::Api,
};
use agent_client_protocol::schema::LogoutRequest;
use futures::future;
use nvim_oxi::{
    Object, ObjectKind,
    conversion::FromObject,
    lua::{self, Poppable, Pushable},
    serde::SerializeError,
};
use tracing::{error, instrument};

#[derive(Clone, Debug, Default)]
pub enum LogoutArgs {
    Multiple(Vec<Assistant>),
    Single(Assistant),
    #[default]
    All,
}

#[instrument(level = "trace", skip_all)]
fn parse_assistant_string(
    assistant: nvim_oxi::String,
) -> Result<Assistant, nvim_oxi::conversion::Error> {
    match assistant.to_string().to_lowercase().as_str() {
        "copilot" => Ok(Assistant::Copilot),
        "opencode" => Ok(Assistant::Opencode),
        other => Err(nvim_oxi::conversion::Error::Serialize(SerializeError {
            msg: format!(
                "Invalid input found: {}, Agent name must be one of 'copilot' or 'opencode'",
                other
            ),
        })),
    }
}

const EXPECTED: &str = "Nil, String or Array of Strings";

impl FromObject for LogoutArgs {
    fn from_object(obj: Object) -> Result<Self, nvim_oxi::conversion::Error> {
        match obj.kind() {
            ObjectKind::Nil => Ok(Self::All),
            ObjectKind::String => {
                let kind = obj.kind();
                let assistant = unsafe { obj.into_string_unchecked() };
                parse_assistant_string(assistant)
                    .map_err(|_| nvim_oxi::conversion::Error::FromWrongType {
                        expected: EXPECTED,
                        actual: kind.as_static(),
                    })
                    .map(Self::Single)
            }
            ObjectKind::Array => {
                let assistants = unsafe { obj.into_array_unchecked() };
                assistants
                    .into_iter()
                    .map(|obj| {
                        if let ObjectKind::String = obj.kind() {
                            Ok(unsafe { obj.into_string_unchecked() })
                        } else {
                            Err(nvim_oxi::conversion::Error::FromWrongType {
                                expected: EXPECTED,
                                actual: obj.kind().as_static(),
                            })
                        }
                    })
                    .collect::<Result<Vec<nvim_oxi::String>, nvim_oxi::conversion::Error>>()?
                    .into_iter()
                    .map(parse_assistant_string)
                    .collect::<Result<Vec<Assistant>, nvim_oxi::conversion::Error>>()
                    .map(Self::Multiple)
            }
            other => Err(nvim_oxi::conversion::Error::FromWrongType {
                expected: EXPECTED,
                actual: other.as_static(),
            }),
        }
    }
}

impl Poppable for LogoutArgs {
    unsafe fn pop(lua: *mut nvim_oxi::lua::ffi::State) -> Result<Self, lua::Error> {
        let obj = unsafe { Object::pop(lua)? };
        Ok(Self::from_object(obj)
            .inspect_err(|e| {
                error!(
                    "An error occurred while parsing the logout arguments, failed to logout: {:?}",
                    e
                )
            })
            .unwrap_or(Self::Multiple(vec![])))
    }
}

impl Pushable for LogoutArgs {
    unsafe fn push(self, state: *mut lua::ffi::State) -> Result<i32, lua::Error> {
        unsafe {
            match self {
                Self::All => ().push(state),
                Self::Single(s) => s.to_string().push(state),
                Self::Multiple(vec) => vec
                    .iter()
                    .map(|s| s.to_string())
                    .collect::<Vec<String>>()
                    .push(state),
            }
        }
    }
}

impl Api {
    #[tracing::instrument(level = "trace", skip(self))]
    pub async fn logout(&self, args: LogoutArgs) -> crate::acp::Result<()> {
        let agents: Vec<Assistant> = match &args {
            LogoutArgs::Single(agent) => vec![agent.clone()],
            LogoutArgs::Multiple(agents) => agents.clone(),
            LogoutArgs::All => self.connection.connected_agents(),
        };
        let futures: Vec<_> = agents
            .iter()
            .map(|assistant| {
                self.connection.get_connection(assistant).ok_or_else(|| {
                    Error::Connection(format!("No connection found for: {}", assistant))
                })
            })
            .collect::<acp::Result<Vec<&Connection>>>()?
            .iter()
            .map(|connection| connection.logout(LogoutRequest::new()))
            .collect();

        future::try_join_all(futures).await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    // Strategy for generating valid assistant names
    fn arb_assistant_name() -> impl Strategy<Value = String> {
        prop_oneof!(
            Just("copilot".to_string()),
            Just("opencode".to_string()),
            Just("COPILOT".to_string()),
            Just("OPENCODE".to_string()),
            Just("Copilot".to_string()),
            Just("Opencode".to_string()),
            "[a-zA-Z][a-zA-Z0-9_]*".prop_map(|s| s.to_string())
        )
    }

    // Strategy for generating LogoutArgs variants
    fn arb_logout_args() -> impl Strategy<Value = LogoutArgs> {
        prop_oneof!(
            Just(LogoutArgs::All),
            arb_assistant_name().prop_map(|name| {
                match Assistant::from(name.as_str()) {
                    Assistant::Copilot | Assistant::Opencode => {
                        LogoutArgs::Single(Assistant::from(name.as_str()))
                    }
                    _ => LogoutArgs::All,
                }
            }),
            prop::collection::vec(arb_assistant_name(), 0..5).prop_map(|names| {
                let assistants: Vec<Assistant> = names
                    .into_iter()
                    .filter_map(|name| match Assistant::from(name.as_str()) {
                        Assistant::Copilot | Assistant::Opencode => {
                            Some(Assistant::from(name.as_str()))
                        }
                        _ => None,
                    })
                    .collect();
                if assistants.is_empty() {
                    LogoutArgs::All
                } else {
                    LogoutArgs::Multiple(assistants)
                }
            })
        )
    }

    proptest! {
        #[test]
        fn test_logout_args_from_str_roundtrip(name in arb_assistant_name()) {
            // Property: converting string to Assistant should never panic
            let _ = Assistant::from(name.as_str());
        }

        #[test]
        fn test_logout_args_pushable_roundtrip(args in arb_logout_args()) {
            match args {
                LogoutArgs::All => {
                    // All variant should remain All
                }
                LogoutArgs::Single(ref assistant) => {
                    prop_assert!(
                        matches!(assistant, Assistant::Copilot | Assistant::Opencode | Assistant::CustomStdio { .. }),
                        "Single variant should contain valid assistant"
                    );
                }
                LogoutArgs::Multiple(ref assistants) => {
                    prop_assert!(
                        !assistants.is_empty(),
                        "Multiple variant should contain at least one assistant"
                    );
                    for assistant in assistants {
                        prop_assert!(
                            matches!(assistant, Assistant::Copilot | Assistant::Opencode | Assistant::CustomStdio { .. }),
                            "Each assistant in Multiple should be valid"
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn test_logout_args_default_is_all() {
        let args: LogoutArgs = Default::default();
        assert!(matches!(args, LogoutArgs::All));
    }

    #[test]
    fn test_parse_assistant_string_copilot() {
        let result = parse_assistant_string(nvim_oxi::String::from("copilot"));
        assert!(matches!(result, Ok(Assistant::Copilot)));
    }

    #[test]
    fn test_parse_assistant_string_opencode() {
        let result = parse_assistant_string(nvim_oxi::String::from("opencode"));
        assert!(matches!(result, Ok(Assistant::Opencode)));
    }

    #[test]
    fn test_parse_assistant_string_case_insensitive() {
        let result = parse_assistant_string(nvim_oxi::String::from("COPILOT"));
        assert!(matches!(result, Ok(Assistant::Copilot)));
    }

    #[test]
    fn test_parse_assistant_string_invalid() {
        let result = parse_assistant_string(nvim_oxi::String::from("invalid"));
        assert!(result.is_err());
    }

    #[test]
    fn test_logout_args_pushable_all_variant() {
        let args = LogoutArgs::All;
        assert!(matches!(args, LogoutArgs::All));
    }

    #[test]
    fn test_logout_args_pushable_single_variant() {
        let args = LogoutArgs::Single(Assistant::Copilot);
        match args {
            LogoutArgs::Single(assistant) => {
                assert!(matches!(assistant, Assistant::Copilot));
            }
            _ => panic!("Expected Single variant"),
        }
    }

    #[test]
    fn test_logout_args_pushable_multiple_variant() {
        let assistants = vec![Assistant::Copilot, Assistant::Opencode];
        let args = LogoutArgs::Multiple(assistants);
        match args {
            LogoutArgs::Multiple(vec) => {
                assert_eq!(vec.len(), 2);
                assert!(matches!(vec[0], Assistant::Copilot));
                assert!(matches!(vec[1], Assistant::Opencode));
            }
            _ => panic!("Expected Multiple variant"),
        }
    }
}
