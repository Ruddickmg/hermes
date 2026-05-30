use crate::{
    acp::{
        Result,
        connection::{Assistant, ConnectionDetails, Protocol},
        error::Error,
        registry::distribution::Distribution,
    },
    api::Api,
};
use nvim_oxi::{
    Dictionary, ObjectKind,
    lua::{Poppable, Pushable},
};

pub fn parse_agent_connection(
    name: String,
    protocol: Protocol,
    options: Option<Dictionary>,
) -> Result<Assistant> {
    if let Some(dict) = options {
        Ok(match protocol {
            Protocol::Stdio => {
                if let (Some(command), args) = (dict.get("command"), dict.get("args")) {
                    let command_str: nvim_oxi::String = command.clone().try_into()?;
                    let args_arr: nvim_oxi::Array = match args {
                        Some(a) => {
                            if a.kind() == ObjectKind::Array {
                                unsafe { a.clone().into_array_unchecked() }
                            } else {
                                return Err(Error::InvalidInput(
                                    "Expected 'args' to be an array of strings".into(),
                                ));
                            }
                        }
                        None => nvim_oxi::Array::default(),
                    };
                    let parsed_args: Vec<String> = args_arr
                        .into_iter()
                        .map(|v| {
                            v.try_into()
                                .map_err(|e| {
                                    Error::InvalidInput(format!(
                                        "Error occurred parsing stdio arguments: {:?}",
                                        e
                                    ))
                                })
                                .map(|s: nvim_oxi::String| s.to_string())
                        })
                        .collect::<Result<Vec<String>>>()?;
                    if command_str.is_empty() {
                        return Err(Error::InvalidInput(
                            "Command cannot be empty for custom stdio connections".into(),
                        ));
                    }
                    Assistant::CustomStdio {
                        name,
                        command: command_str.to_string(),
                        args: parsed_args,
                    }
                } else {
                    Assistant::from(name.clone())
                }
            }
            _ => {
                if let (Some(host), Some(port)) = (dict.get("host"), dict.get("port")) {
                    let host_value: nvim_oxi::String = host.clone().try_into()?;
                    let port_value: u16 = port.clone().try_into()?;
                    Assistant::CustomUrl {
                        name,
                        host: host_value.to_string(),
                        port: port_value,
                    }
                } else {
                    return Err(Error::InvalidInput(format!(
                        "Host and port must be provided for {} connections",
                        protocol
                    )));
                }
            }
        })
    } else {
        Ok(match Assistant::from(name) {
            Assistant::CustomStdio { .. } => {
                return Err(Error::InvalidInput(
                    "Custom stdio connections require options with a 'command' field".into(),
                ));
            }
            assistant => assistant,
        })
    }
}

fn parse_distribution(dict: &Dictionary) -> Result<Option<Distribution>> {
    match dict.get("distribution") {
        Some(obj) => {
            let s: nvim_oxi::String = obj
                .clone()
                .try_into()
                .map_err(|_| Error::InvalidInput("'distribution' must be a string".into()))?;
            let lower = s.to_string().to_lowercase();
            match Distribution::from(lower) {
                Distribution::Invalid => Err(Error::InvalidInput(format!(
                    "Unknown distribution '{lower}'. Must be one of: npx, uvx, binary"
                ))),
                valid => Ok(Some(valid)),
            }
        }
        None => Ok(None),
    }
}

#[derive(Clone, Copy, Debug)]
pub struct ConnectionOptions {
    protocol: Protocol,
    distribution: Option<Distribution>,
}

impl Poppable for ConnectionOptions {}

impl Pushable for ConnectionOptions {}

pub type ConnectionArgs = (nvim_oxi::String, Option<ConnectionOptions>);

impl Api {
    #[tracing::instrument(level = "trace", skip(self))]
    pub async fn connect(&mut self, (agent_name, options): ConnectionArgs) -> Result<()> {
        let mut protocol = Protocol::default();
        let mut preference = None;
        if let Some(ref dict) = options {
            if let Some(obj) = dict.get("protocol") {
                protocol = obj
                    .clone()
                    .try_into()
                    .map(|s: nvim_oxi::String| Protocol::from(s.to_string()))?;
            }
            preference = parse_distribution(dict)?;
        }
        let agent_id = agent_name.to_string();

        let agent = {
            let state = self.state.lock().await;
            let entry = state
                .registry
                .as_ref()
                .and_then(|r| r.agents.get(&agent_id))
                .cloned();
            drop(state);

            match entry {
                Some(agent) => Assistant::Registered {
                    agent,
                    distribution: preference,
                },
                None => parse_agent_connection(agent_id, protocol, options)?,
            }
        };

        self.connection
            .connect(
                self.response_handler.clone(),
                ConnectionDetails { agent, protocol },
            )
            .await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    // Strategy for generating agent names
    fn arb_agent_name() -> impl Strategy<Value = String> {
        prop_oneof!(
            Just("copilot".to_string()),
            Just("opencode".to_string()),
            Just("COPILOT".to_string()),
            Just("OPENCODE".to_string()),
            Just("Copilot".to_string()),
            Just("Opencode".to_string()),
            "[a-zA-Z][a-zA-Z0-9_-]*".prop_map(|s| s.to_string())
        )
    }

    proptest! {
        #[test]
        fn test_assistant_from_str_never_panics(name in arb_agent_name()) {
            // Property: converting any string to Assistant should never panic
            let _ = Assistant::from(name.as_str());
        }

        #[test]
        fn test_known_agents_parsed_correctly(name in prop_oneof!(
            Just("copilot"),
            Just("opencode"),
            Just("COPILOT"),
            Just("OPENCODE")
        )) {
            // Property: Known agents should parse to their respective variants
            let assistant = Assistant::from(name);
            let name_lower = name.to_lowercase();

            if name_lower == "copilot" {
                prop_assert!(matches!(assistant, Assistant::Copilot), "Expected Copilot variant");
            } else if name_lower == "opencode" {
                prop_assert!(matches!(assistant, Assistant::Opencode), "Expected Opencode variant");
            }
        }
    }

    #[test]
    fn test_assistant_from_str_copilot() {
        assert!(matches!(Assistant::from("copilot"), Assistant::Copilot));
    }

    #[test]
    fn test_assistant_from_str_opencode() {
        assert!(matches!(Assistant::from("opencode"), Assistant::Opencode));
    }

    #[test]
    fn test_assistant_from_str_case_insensitive() {
        assert!(matches!(Assistant::from("COPILOT"), Assistant::Copilot));
        assert!(matches!(Assistant::from("OpEnCoDe"), Assistant::Opencode));
    }

    #[test]
    fn test_assistant_custom_with_command() {
        let assistant = Assistant::from("custom-agent");
        assert!(matches!(assistant, Assistant::CustomStdio { name, .. } if name == "custom-agent"));
    }

    // Tests for parse_agent_connection function
    #[test]
    fn test_parse_agent_connection_stdio_with_command() {
        let mut dict = Dictionary::new();
        dict.insert("command", "my-agent");
        dict.insert("args", nvim_oxi::Array::from_iter(["arg1", "arg2"]));

        let result = parse_agent_connection("test-agent".to_string(), Protocol::Stdio, Some(dict));
        assert!(result.is_ok());

        let assistant = result.unwrap();
        assert!(
            matches!(assistant, Assistant::CustomStdio { name, command, args } 
            if name == "test-agent" && command == "my-agent" && args == vec!["arg1", "arg2"])
        );
    }

    #[test]
    fn test_parse_agent_connection_stdio_without_options() {
        let result = parse_agent_connection("copilot".to_string(), Protocol::Stdio, None);
        assert!(result.is_ok());
        assert!(matches!(result.unwrap(), Assistant::Copilot));
    }

    #[test]
    fn test_parse_agent_connection_stdio_without_command() {
        let dict = Dictionary::new();
        let result = parse_agent_connection("opencode".to_string(), Protocol::Stdio, Some(dict));
        assert!(result.is_ok());
        assert!(matches!(result.unwrap(), Assistant::Opencode));
    }

    #[test]
    fn test_parse_agent_connection_socket_with_host_port() {
        let mut dict = Dictionary::new();
        dict.insert("host", "localhost");
        dict.insert("port", 8080i64);

        let result =
            parse_agent_connection("socket-agent".to_string(), Protocol::Socket, Some(dict));
        assert!(result.is_ok());

        let assistant = result.unwrap();
        assert!(
            matches!(assistant, Assistant::CustomUrl { name, host, port } 
            if name == "socket-agent" && host == "localhost" && port == 8080)
        );
    }

    #[test]
    fn test_parse_agent_connection_http_with_host_port() {
        let mut dict = Dictionary::new();
        dict.insert("host", "api.example.com");
        dict.insert("port", 443i64);

        let result = parse_agent_connection("http-agent".to_string(), Protocol::Http, Some(dict));
        assert!(result.is_ok());

        let assistant = result.unwrap();
        assert!(
            matches!(assistant, Assistant::CustomUrl { name: _, host, port } 
            if host == "api.example.com" && port == 443)
        );
    }

    #[test]
    fn test_parse_agent_connection_socket_missing_host() {
        let mut dict = Dictionary::new();
        dict.insert("port", 8080i64);

        let result =
            parse_agent_connection("socket-agent".to_string(), Protocol::Socket, Some(dict));
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_agent_connection_socket_missing_port() {
        let mut dict = Dictionary::new();
        dict.insert("host", "localhost");

        let result = parse_agent_connection("socket-agent".to_string(), Protocol::Tcp, Some(dict));
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("Host and port must be provided")
        );
    }

    #[test]
    fn test_parse_agent_connection_stdio_empty_command() {
        let mut dict = Dictionary::new();
        dict.insert("command", "");

        let result = parse_agent_connection("test-agent".to_string(), Protocol::Stdio, Some(dict));
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("Command cannot be empty")
        );
    }

    #[test]
    fn test_parse_agent_connection_stdio_invalid_args_type() {
        let mut dict = Dictionary::new();
        dict.insert("command", "my-agent");
        dict.insert("args", "not-an-array");

        let result = parse_agent_connection("test-agent".to_string(), Protocol::Stdio, Some(dict));
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("Expected 'args' to be an array")
        );
    }

    // Tests for Protocol parsing
    #[test]
    fn test_protocol_from_str_stdio() {
        assert!(matches!(Protocol::from("stdio"), Protocol::Stdio));
        assert!(matches!(Protocol::from("STDIO"), Protocol::Stdio));
        assert!(matches!(Protocol::from("Stdio"), Protocol::Stdio));
    }

    #[test]
    fn test_protocol_from_str_socket() {
        assert!(matches!(Protocol::from("socket"), Protocol::Socket));
        assert!(matches!(Protocol::from("SOCKET"), Protocol::Socket));
    }

    #[test]
    fn test_protocol_from_str_http() {
        assert!(matches!(Protocol::from("http"), Protocol::Http));
        assert!(matches!(Protocol::from("HTTP"), Protocol::Http));
    }

    #[test]
    fn test_protocol_from_str_unknown_defaults_to_stdio() {
        assert!(matches!(Protocol::from("unknown"), Protocol::Stdio));
        assert!(matches!(Protocol::from(""), Protocol::Stdio));
    }

    // Tests for Protocol Display trait
    #[test]
    fn test_protocol_display_stdio() {
        assert_eq!(format!("{}", Protocol::Stdio), "stdio");
    }

    #[test]
    fn test_protocol_display_socket() {
        assert_eq!(format!("{}", Protocol::Socket), "socket");
    }

    #[test]
    fn test_protocol_display_http() {
        assert_eq!(format!("{}", Protocol::Http), "http");
    }

    // Proptest for protocol parsing
    proptest! {
        #[test]
        fn test_protocol_from_str_never_panics(input in "[a-zA-Z0-9]*") {
            // Property: converting any string to Protocol should never panic
            let _ = Protocol::from(input.as_str());
        }

        #[test]
        fn test_protocol_stdio_case_insensitive(
            variant in "(stdio|STDIO|Stdio|StDiO)"
        ) {
            let protocol = Protocol::from(variant.as_str());
            prop_assert!(matches!(protocol, Protocol::Stdio));
        }

        #[test]
        fn test_protocol_socket_case_insensitive(
            variant in "(socket|SOCKET|Socket|SoCkEt)"
        ) {
            let protocol = Protocol::from(variant.as_str());
            prop_assert!(matches!(protocol, Protocol::Socket));
        }

        #[test]
        fn test_protocol_http_case_insensitive(
            variant in "(http|HTTP|Http|HtTp)"
        ) {
            let protocol = Protocol::from(variant.as_str());
            prop_assert!(matches!(protocol, Protocol::Http));
        }
    }

    // Integration-style tests for the connect function
    // Note: These would require mocking the ConnectionManager in integration tests
    #[test]
    fn test_connection_args_type() {
        // Verify the ConnectionArgs type is correctly defined
        let _: ConnectionArgs = (nvim_oxi::String::from("copilot"), None);
        let mut dict = Dictionary::new();
        dict.insert("protocol", "socket");
        let _: ConnectionArgs = (nvim_oxi::String::from("test"), Some(dict));
    }

    // Tests for parse_distribution
    #[test]
    fn parse_distribution_npx() {
        let mut dict = Dictionary::new();
        dict.insert("distribution", "npx");
        let result = parse_distribution(&dict);
        assert_eq!(result.unwrap(), Some("npx".to_string()));
    }

    #[test]
    fn parse_distribution_uvx() {
        let mut dict = Dictionary::new();
        dict.insert("distribution", "uvx");
        let result = parse_distribution(&dict);
        assert_eq!(result.unwrap(), Some("uvx".to_string()));
    }

    #[test]
    fn parse_distribution_binary() {
        let mut dict = Dictionary::new();
        dict.insert("distribution", "binary");
        let result = parse_distribution(&dict);
        assert_eq!(result.unwrap(), Some("binary".to_string()));
    }

    #[test]
    fn parse_distribution_case_insensitive() {
        let mut dict = Dictionary::new();
        dict.insert("distribution", "NPX");
        let result = parse_distribution(&dict);
        assert_eq!(result.unwrap(), Some("npx".to_string()));
    }

    #[test]
    fn parse_distribution_missing() {
        let dict = Dictionary::new();
        let result = parse_distribution(&dict);
        assert!(result.unwrap().is_none());
    }

    #[test]
    fn parse_distribution_invalid_value() {
        let mut dict = Dictionary::new();
        dict.insert("distribution", "bad");
        let result = parse_distribution(&dict);
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("Unknown distribution"),
            "Error should describe unknown distribution"
        );
    }

    #[test]
    fn parse_distribution_non_string() {
        let mut dict = Dictionary::new();
        dict.insert("distribution", 42i64);
        let result = parse_distribution(&dict);
        assert!(result.is_err());
        assert!(
            result.unwrap_err().to_string().contains("must be a string"),
            "Error should indicate distribution must be a string"
        );
    }
}
