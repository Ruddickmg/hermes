use std::time::Duration;

use crate::{
    TIMEOUT_IN_SECONDS,
    utilities::{autocommand, mock_agent::MockAgent, mock_config::generate_session_id},
};
use agent_client_protocol::schema::{
    InitializeResponse, NewSessionResponse, SessionConfigOption, SessionConfigOptionCategory,
    SessionConfigSelectOption, SessionMode, SessionModeState,
};
use hermes::{
    api::{ConnectionArgs, CreateSessionArgs, DisconnectArgs},
    nvim::{autocommands::Commands, hermes},
};
use nvim_oxi::{Array, Dictionary, Function, conversion::FromObject};

#[nvim_oxi::test]
fn test_setup_returns_modes_function() -> Result<(), nvim_oxi::Error> {
    let dict: Dictionary = hermes()?;

    assert!(
        dict.get("modes").is_some(),
        "modes function should be registered"
    );

    Ok(())
}

#[nvim_oxi::test]
fn test_modes_returns_nil_when_no_session() -> Result<(), nvim_oxi::Error> {
    let dict: Dictionary = hermes()?;
    let modes: Function<String, Option<Array>> =
        FromObject::from_object(dict.get("modes").unwrap().clone())?;

    let result = modes.call("nonexistent-session".to_string());

    assert_eq!(
        result,
        Ok(None),
        "modes should return nil when session not found"
    );

    Ok(())
}

#[nvim_oxi::test]
fn test_modes_returns_legacy_modes() -> Result<(), nvim_oxi::Error> {
    let dict: Dictionary = hermes()?;
    let connect: Function<ConnectionArgs, ()> =
        FromObject::from_object(dict.get("connect").unwrap().clone())?;
    let disconnect: Function<DisconnectArgs, ()> =
        FromObject::from_object(dict.get("disconnect").unwrap().clone())?;
    let create_session: Function<CreateSessionArgs, ()> =
        FromObject::from_object(dict.get("create_session").unwrap().clone())?;
    let modes: Function<String, Option<Array>> =
        FromObject::from_object(dict.get("modes").unwrap().clone())?;

    let wait_for_initialization =
        autocommand::listen_for_autocommand::<InitializeResponse>(Commands::ConnectionInitialized);
    let wait_for_session =
        autocommand::listen_for_autocommand::<NewSessionResponse>(Commands::SessionCreated);

    let (agent, conn_rx) = MockAgent::new();
    {
        let mut config = agent.config().lock().unwrap();
        let mode = SessionMode::new("chat", "Chat");
        let modes = SessionModeState::new("chat", vec![mode]);
        config.new_session_response = NewSessionResponse::new(generate_session_id()).modes(modes);
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

    let result = modes.call(session_id);

    disconnect.call(DisconnectArgs::All)?;
    mock_handle.close();

    assert!(
        result.is_ok(),
        "modes should succeed for legacy modes session"
    );
    let maybe_array = result.unwrap();
    assert!(maybe_array.is_some(), "modes should return array, not nil");
    let array = maybe_array.unwrap();
    assert_eq!(array.len(), 1, "Should return one mode");

    Ok(())
}

#[nvim_oxi::test]
fn test_modes_returns_config_options() -> Result<(), nvim_oxi::Error> {
    let dict: Dictionary = hermes()?;
    let connect: Function<ConnectionArgs, ()> =
        FromObject::from_object(dict.get("connect").unwrap().clone())?;
    let disconnect: Function<DisconnectArgs, ()> =
        FromObject::from_object(dict.get("disconnect").unwrap().clone())?;
    let create_session: Function<CreateSessionArgs, ()> =
        FromObject::from_object(dict.get("create_session").unwrap().clone())?;
    let modes: Function<String, Option<Array>> =
        FromObject::from_object(dict.get("modes").unwrap().clone())?;

    let wait_for_initialization =
        autocommand::listen_for_autocommand::<InitializeResponse>(Commands::ConnectionInitialized);
    let wait_for_session =
        autocommand::listen_for_autocommand::<NewSessionResponse>(Commands::SessionCreated);

    let (agent, conn_rx) = MockAgent::new();
    {
        let mut config = agent.config().lock().unwrap();
        let option = SessionConfigOption::select(
            "mode",
            "Mode",
            "chat",
            vec![SessionConfigSelectOption::new("chat", "Chat")],
        )
        .category(SessionConfigOptionCategory::Mode);
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

    let result = modes.call(session_id);

    disconnect.call(DisconnectArgs::All)?;
    mock_handle.close();

    assert!(
        result.is_ok(),
        "modes should succeed for config options session"
    );
    let maybe_array = result.unwrap();
    assert!(maybe_array.is_some(), "modes should return array, not nil");
    let array = maybe_array.unwrap();
    assert_eq!(array.len(), 1, "Should return one mode");

    Ok(())
}

#[nvim_oxi::test]
fn test_modes_returns_grouped_options() -> Result<(), nvim_oxi::Error> {
    let dict: Dictionary = hermes()?;
    let connect: Function<ConnectionArgs, ()> =
        FromObject::from_object(dict.get("connect").unwrap().clone())?;
    let disconnect: Function<DisconnectArgs, ()> =
        FromObject::from_object(dict.get("disconnect").unwrap().clone())?;
    let create_session: Function<CreateSessionArgs, ()> =
        FromObject::from_object(dict.get("create_session").unwrap().clone())?;
    let modes: Function<String, Option<Array>> =
        FromObject::from_object(dict.get("modes").unwrap().clone())?;

    let wait_for_initialization =
        autocommand::listen_for_autocommand::<InitializeResponse>(Commands::ConnectionInitialized);
    let wait_for_session =
        autocommand::listen_for_autocommand::<NewSessionResponse>(Commands::SessionCreated);

    let (agent, conn_rx) = MockAgent::new();
    {
        let mut config = agent.config().lock().unwrap();
        let group = agent_client_protocol::schema::SessionConfigSelectGroup::new(
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

    let result = modes.call(session_id);

    disconnect.call(DisconnectArgs::All)?;
    mock_handle.close();

    assert!(
        result.is_ok(),
        "modes should succeed for grouped options session"
    );
    let maybe_array = result.unwrap();
    assert!(maybe_array.is_some(), "modes should return array, not nil");
    let array = maybe_array.unwrap();
    assert_eq!(array.len(), 1, "Should return one mode");

    let obj = array.get(0).expect("Array should have one element");
    let mode_dict: Dictionary = obj.clone().try_into().expect("Should be a dictionary");
    let group_value: nvim_oxi::String = mode_dict
        .get("group")
        .expect("Should have group key")
        .clone()
        .try_into()
        .expect("Should be a string");
    assert_eq!(group_value.to_string(), "My Group");

    Ok(())
}
