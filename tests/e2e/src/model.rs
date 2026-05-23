use crate::{
    TIMEOUT_IN_SECONDS,
    utilities::{autocommand, mock_agent::MockAgent, mock_config::generate_session_id},
};
use agent_client_protocol::schema::{
    InitializeResponse, NewSessionResponse, SessionConfigOption, SessionConfigOptionCategory,
    SessionConfigSelectOption,
};
use hermes::{
    api::{ConnectionArgs, CreateSessionArgs, DisconnectArgs, SetModelArgs},
    nvim::{autocommands::Commands, hermes},
};
use nvim_oxi::{Dictionary, Function, conversion::FromObject};
use std::time::Duration;

#[nvim_oxi::test]
fn test_setup_returns_set_model_function() -> Result<(), nvim_oxi::Error> {
    let dict: Dictionary = hermes()?;

    assert!(
        dict.get("set_model").is_some(),
        "set_model function should be registered"
    );

    Ok(())
}

// Mock agent doesn't support session models (returns None for models),
// so set_model should do nothing and return Ok when session_info has no models entry.
#[nvim_oxi::test]
fn test_set_model_no_models_does_not_crash() -> Result<(), nvim_oxi::Error> {
    let dict: Dictionary = hermes()?;
    let connect: Function<ConnectionArgs, ()> =
        FromObject::from_object(dict.get("connect").unwrap().clone())?;
    let disconnect: Function<DisconnectArgs, ()> =
        FromObject::from_object(dict.get("disconnect").unwrap().clone())?;
    let create_session: Function<CreateSessionArgs, ()> =
        FromObject::from_object(dict.get("create_session").unwrap().clone())?;
    let set_model: Function<SetModelArgs, ()> =
        FromObject::from_object(dict.get("set_model").unwrap().clone())?;

    let wait_for_initialization =
        autocommand::listen_for_autocommand::<InitializeResponse>(Commands::ConnectionInitialized);
    let wait_for_session =
        autocommand::listen_for_autocommand::<NewSessionResponse>(Commands::SessionCreated);

    // Start mock agent
    let (agent, conn_rx) = MockAgent::new();
    let mock_handle = MockAgent::start(agent, conn_rx).expect("Failed to start mock agent");

    let mut options = Dictionary::new();
    options.insert("protocol", "tcp");
    options.insert("host", "localhost");
    options.insert("port", mock_handle.port() as i64);

    connect.call((nvim_oxi::String::from("mock-agent"), Some(options)))?;

    wait_for_initialization(Duration::from_secs(TIMEOUT_IN_SECONDS))?;

    create_session.call(CreateSessionArgs::Default)?;

    let session = wait_for_session(Duration::from_secs(TIMEOUT_IN_SECONDS))?;
    let session_id = session.session_id;

    // session_info has no models entry, so set_model does nothing and returns Ok
    let result = set_model.call((session_id.to_string(), "gpt4".to_string()));

    disconnect.call(DisconnectArgs::All)?;
    mock_handle.close();

    assert_eq!(
        result,
        Ok(()),
        "set_model should return Ok when no models are configured"
    );

    Ok(())
}

// Configure mock agent to return legacy session.models.
// set_model should call connection.set_session_model(...) which the mock agent handles.
#[nvim_oxi::test]
fn test_set_model_with_legacy_models() -> Result<(), nvim_oxi::Error> {
    let dict: Dictionary = hermes()?;
    let connect: Function<ConnectionArgs, ()> =
        FromObject::from_object(dict.get("connect").unwrap().clone())?;
    let disconnect: Function<DisconnectArgs, ()> =
        FromObject::from_object(dict.get("disconnect").unwrap().clone())?;
    let create_session: Function<CreateSessionArgs, ()> =
        FromObject::from_object(dict.get("create_session").unwrap().clone())?;
    let set_model: Function<SetModelArgs, ()> =
        FromObject::from_object(dict.get("set_model").unwrap().clone())?;

    let wait_for_initialization =
        autocommand::listen_for_autocommand::<InitializeResponse>(Commands::ConnectionInitialized);
    let wait_for_session =
        autocommand::listen_for_autocommand::<NewSessionResponse>(Commands::SessionCreated);

    // Configure mock agent with legacy models
    let (agent, conn_rx) = MockAgent::new();
    {
        let mut config = agent.config().lock().unwrap();
        let model = agent_client_protocol::schema::ModelInfo::new("gpt4", "GPT-4");
        let models = agent_client_protocol::schema::SessionModelState::new("gpt4", vec![model]);
        config.new_session_response = NewSessionResponse::new(generate_session_id()).models(models);
        config.set_session_model_response =
            Some(agent_client_protocol::schema::SetSessionModelResponse::new());
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
    let session_id = session.session_id;

    let result = set_model.call((session_id.to_string(), "gpt4".to_string()));

    disconnect.call(DisconnectArgs::All)?;
    mock_handle.close();

    assert_eq!(
        result,
        Ok(()),
        "set_model with legacy models should succeed"
    );

    Ok(())
}

// Configure mock agent to return new config_options with Model category.
// set_model should call connection.set_config_option(...) which the mock agent handles.
#[nvim_oxi::test]
fn test_set_model_with_config_options() -> Result<(), nvim_oxi::Error> {
    let dict: Dictionary = hermes()?;
    let connect: Function<ConnectionArgs, ()> =
        FromObject::from_object(dict.get("connect").unwrap().clone())?;
    let disconnect: Function<DisconnectArgs, ()> =
        FromObject::from_object(dict.get("disconnect").unwrap().clone())?;
    let create_session: Function<CreateSessionArgs, ()> =
        FromObject::from_object(dict.get("create_session").unwrap().clone())?;
    let set_model: Function<SetModelArgs, ()> =
        FromObject::from_object(dict.get("set_model").unwrap().clone())?;

    let wait_for_initialization =
        autocommand::listen_for_autocommand::<InitializeResponse>(Commands::ConnectionInitialized);
    let wait_for_session =
        autocommand::listen_for_autocommand::<NewSessionResponse>(Commands::SessionCreated);

    // Configure mock agent with new config options path
    let (agent, conn_rx) = MockAgent::new();
    {
        let mut config = agent.config().lock().unwrap();
        let option = SessionConfigOption::select(
            "model",
            "Model",
            "gpt4",
            vec![SessionConfigSelectOption::new("gpt4", "GPT-4")],
        )
        .category(SessionConfigOptionCategory::Model);
        config.new_session_response =
            NewSessionResponse::new(generate_session_id()).config_options(vec![option]);
        config.set_session_config_option_response =
            Some(agent_client_protocol::schema::SetSessionConfigOptionResponse::new(vec![]));
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
    let session_id = session.session_id;

    let result = set_model.call((session_id.to_string(), "gpt4".to_string()));

    disconnect.call(DisconnectArgs::All)?;
    mock_handle.close();

    assert_eq!(
        result,
        Ok(()),
        "set_model with config options should succeed"
    );

    Ok(())
}
