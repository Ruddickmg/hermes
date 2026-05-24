use std::{collections::HashMap, time::Duration};

use crate::{
    TIMEOUT_IN_SECONDS,
    utilities::{autocommand, mock_agent::MockAgent, mock_config::generate_session_id},
};
use agent_client_protocol::schema::{
    InitializeResponse, NewSessionResponse, SessionConfigOption, SessionConfigOptionCategory,
    SessionConfigSelectOption,
};
use hermes::{
    api::{ConnectionArgs, CreateSessionArgs, DisconnectArgs},
    nvim::{autocommands::Commands, hermes},
};
use nvim_oxi::{Array, Dictionary, Function, Object, conversion::FromObject};
use pretty_assertions::assert_eq;

#[nvim_oxi::test]
fn test_setup_returns_thought_levels_function() -> Result<(), nvim_oxi::Error> {
    let dict: Dictionary = hermes()?;

    assert!(
        dict.get("thought_levels").is_some(),
        "thought_levels function should be registered"
    );

    Ok(())
}

#[nvim_oxi::test]
fn test_thought_levels_returns_nil_when_no_session() -> Result<(), nvim_oxi::Error> {
    let dict: Dictionary = hermes()?;
    let thought_levels: Function<String, Option<Array>> =
        FromObject::from_object(dict.get("thought_levels").unwrap().clone())?;

    let result = thought_levels.call("nonexistent-session".to_string());

    assert_eq!(
        result,
        Ok(None),
        "thought_levels should return nil when session not found"
    );

    Ok(())
}

fn dict_to_hashmap(dict: Dictionary) -> HashMap<String, String> {
    dict.into_iter().fold(HashMap::new(), |mut acc, (k, v)| {
        let s: nvim_oxi::String = v.try_into().unwrap();
        acc.insert(k.to_string(), s.to_string());
        acc
    })
}

fn hashmap_thought_level(value: &str, name: &str) -> HashMap<String, String> {
    let mut map = HashMap::new();
    map.insert("value".to_string(), value.to_string());
    map.insert("name".to_string(), name.to_string());
    map
}

fn collect_thought_levels(result: Option<Array>) -> Vec<HashMap<String, String>> {
    result
        .unwrap()
        .into_iter()
        .map(|obj| {
            let dict: Dictionary = obj.try_into().expect("Object should be a dictionary");
            dict_to_hashmap(dict)
        })
        .collect()
}

#[nvim_oxi::test]
fn test_thought_levels_returns_config_options() -> Result<(), nvim_oxi::Error> {
    let dict: Dictionary = hermes()?;
    let connect: Function<ConnectionArgs, ()> =
        FromObject::from_object(dict.get("connect").unwrap().clone())?;
    let disconnect: Function<DisconnectArgs, ()> =
        FromObject::from_object(dict.get("disconnect").unwrap().clone())?;
    let create_session: Function<CreateSessionArgs, ()> =
        FromObject::from_object(dict.get("create_session").unwrap().clone())?;
    let thought_levels: Function<String, Option<Array>> =
        FromObject::from_object(dict.get("thought_levels").unwrap().clone())?;

    let wait_for_initialization =
        autocommand::listen_for_autocommand::<InitializeResponse>(Commands::ConnectionInitialized);
    let wait_for_session =
        autocommand::listen_for_autocommand::<NewSessionResponse>(Commands::SessionCreated);

    let (agent, conn_rx) = MockAgent::new();
    {
        let mut config = agent.config().lock().unwrap();
        let option = SessionConfigOption::select(
            "thought_level",
            "Thought Level",
            "low",
            vec![
                SessionConfigSelectOption::new("low", "Low"),
                SessionConfigSelectOption::new("medium", "Medium"),
            ],
        )
        .category(SessionConfigOptionCategory::ThoughtLevel);
        config.new_session_response =
            NewSessionResponse::new(generate_session_id()).config_options(vec![option]);
    }
    let mock_handle = MockAgent::start(agent, conn_rx).expect("Failed to start mock agent");

    let mut options = Dictionary::new();
    options.insert("protocol", "tcp");
    options.insert("host", "localhost");
    options.insert("port", mock_handle.port() as i64);

    connect.call((nvim_oxi::String::from("mock-agent"), Some(options)))?;

    wait_for_initialization(Duration::from_secs(TIMEOUT_IN_SECONDS))?;

    create_session.call(CreateSessionArgs::Default)?;

    let session = wait_for_session(Duration::from_secs(TIMEOUT_IN_SECONDS))?;
    let session_id = session.session_id.to_string();

    let result = thought_levels.call(session_id)?;

    disconnect.call(DisconnectArgs::All)?;
    mock_handle.close();

    assert_eq!(
        collect_thought_levels(result),
        vec![
            hashmap_thought_level("low", "Low"),
            hashmap_thought_level("medium", "Medium"),
        ]
    );

    Ok(())
}
