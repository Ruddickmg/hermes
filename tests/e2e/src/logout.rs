use hermes::{
    acp::connection::Assistant,
    api::{ConnectionArgs, LogoutArgs},
    nvim::autocommands::Commands,
};
use nvim_oxi::{Dictionary, Function, conversion::FromObject};
use std::time::Duration;

use crate::{
    TIMEOUT_IN_SECONDS,
    utilities::{autocommand, mock_agent::MockAgent},
};

#[nvim_oxi::test]
fn test_logout_function() -> Result<(), nvim_oxi::Error> {
    let dict: Dictionary = hermes::nvim::hermes()?;

    let connect_obj = dict.get("connect").expect("connect function not found");
    let connect: Function<ConnectionArgs, ()> = FromObject::from_object(connect_obj.clone())?;

    let logout_obj = dict.get("logout").expect("logout function not found");
    let logout: Function<LogoutArgs, ()> = FromObject::from_object(logout_obj.clone())?;

    let disconnect_obj = dict
        .get("disconnect")
        .expect("disconnect function not found");
    let disconnect: Function<hermes::api::DisconnectArgs, ()> =
        FromObject::from_object(disconnect_obj.clone())?;

    // Start mock agent for this test
    let (agent, conn_rx) = MockAgent::new();
    let mock_handle = MockAgent::start(agent, conn_rx).expect("Failed to start mock agent");

    let mut options = Dictionary::new();
    options.insert("protocol", "tcp");
    options.insert("host", "localhost");
    options.insert("port", mock_handle.port() as i64);

    connect.call((nvim_oxi::String::from("mock-agent"), Some(options)))?;

    // Wait for initialization before logging out
    let wait_for_init = autocommand::listen_for_autocommand::<
        agent_client_protocol::schema::InitializeResponse,
    >(Commands::ConnectionInitialized);
    let _ = wait_for_init(Duration::from_secs(TIMEOUT_IN_SECONDS))?;

    let wait_for_logout = autocommand::listen_for_autocommand::<
        agent_client_protocol::schema::LogoutResponse,
    >(Commands::LoggedOut);

    // Logout should succeed
    logout.call(LogoutArgs::All).unwrap();

    let result = wait_for_logout(Duration::from_secs(TIMEOUT_IN_SECONDS));

    assert!(result.is_ok(), "logout should succeed");

    // Cleanup
    disconnect.call(hermes::api::DisconnectArgs::All)?;
    mock_handle.close();

    Ok(())
}

#[nvim_oxi::test]
fn test_logout_single() -> Result<(), nvim_oxi::Error> {
    let dict: Dictionary = hermes::nvim::hermes()?;

    let connect_obj = dict.get("connect").expect("connect function not found");
    let connect: Function<ConnectionArgs, ()> = FromObject::from_object(connect_obj.clone())?;

    let logout_obj = dict.get("logout").expect("logout function not found");
    let logout: Function<LogoutArgs, ()> = FromObject::from_object(logout_obj.clone())?;

    let disconnect_obj = dict
        .get("disconnect")
        .expect("disconnect function not found");
    let disconnect: Function<hermes::api::DisconnectArgs, ()> =
        FromObject::from_object(disconnect_obj.clone())?;

    // Start mock agent for this test
    let (agent, conn_rx) = MockAgent::new();
    let mock_handle = MockAgent::start(agent, conn_rx).expect("Failed to start mock agent");

    let mut options = Dictionary::new();
    options.insert("protocol", "tcp");
    options.insert("host", "localhost");
    options.insert("port", mock_handle.port() as i64);

    connect.call((nvim_oxi::String::from("mock-agent"), Some(options)))?;

    // Wait for initialization before logging out
    let wait_for_init = autocommand::listen_for_autocommand::<
        agent_client_protocol::schema::InitializeResponse,
    >(Commands::ConnectionInitialized);
    let _ = wait_for_init(Duration::from_secs(TIMEOUT_IN_SECONDS))?;

    let wait_for_logout = autocommand::listen_for_autocommand::<
        agent_client_protocol::schema::LogoutResponse,
    >(Commands::LoggedOut);

    // Logout single agent by name
    logout.call(LogoutArgs::Single(Assistant::from("mock-agent")))?;

    let result = wait_for_logout(Duration::from_secs(TIMEOUT_IN_SECONDS));

    assert!(result.is_ok(), "logout single by name should succeed");

    // Cleanup
    disconnect.call(hermes::api::DisconnectArgs::All)?;
    mock_handle.close();

    Ok(())
}

#[nvim_oxi::test]
fn test_logout_multiple() -> Result<(), nvim_oxi::Error> {
    let dict: Dictionary = hermes::nvim::hermes()?;

    let connect_obj = dict.get("connect").expect("connect function not found");
    let connect: Function<ConnectionArgs, ()> = FromObject::from_object(connect_obj.clone())?;

    let logout_obj = dict.get("logout").expect("logout function not found");
    let logout: Function<LogoutArgs, ()> = FromObject::from_object(logout_obj.clone())?;

    let disconnect_obj = dict
        .get("disconnect")
        .expect("disconnect function not found");
    let disconnect: Function<hermes::api::DisconnectArgs, ()> =
        FromObject::from_object(disconnect_obj.clone())?;

    // Start mock agent for this test
    let (agent, conn_rx) = MockAgent::new();
    let mock_handle = MockAgent::start(agent, conn_rx).expect("Failed to start mock agent");

    let mut options = Dictionary::new();
    options.insert("protocol", "tcp");
    options.insert("host", "localhost");
    options.insert("port", mock_handle.port() as i64);

    connect.call((nvim_oxi::String::from("mock-agent"), Some(options)))?;

    // Wait for initialization before logging out
    let wait_for_init = autocommand::listen_for_autocommand::<
        agent_client_protocol::schema::InitializeResponse,
    >(Commands::ConnectionInitialized);
    let _ = wait_for_init(Duration::from_secs(TIMEOUT_IN_SECONDS))?;

    let wait_for_logout = autocommand::listen_for_autocommand::<
        agent_client_protocol::schema::LogoutResponse,
    >(Commands::LoggedOut);

    // Logout multiple agents including the connected one
    logout.call(LogoutArgs::Multiple(vec![Assistant::from("mock-agent")]))?;

    let result = wait_for_logout(Duration::from_secs(TIMEOUT_IN_SECONDS));

    assert!(result.is_ok(), "logout multiple by name should succeed");

    // Cleanup
    disconnect.call(hermes::api::DisconnectArgs::All)?;
    mock_handle.close();

    Ok(())
}
