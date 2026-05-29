use hermes::{acp::connection::Assistant, api::ConnectionArgs, nvim::autocommands::Commands};
use nvim_oxi::{Dictionary, Function, conversion::FromObject};
use std::time::Duration;

use crate::{
    TIMEOUT_IN_SECONDS,
    utilities::{autocommand, mock_agent::MockAgent},
};

#[nvim_oxi::test]
fn test_disconnect_all() -> Result<(), nvim_oxi::Error> {
    let dict: Dictionary = hermes::nvim::hermes()?;

    let connect: Function<ConnectionArgs, ()> = FromObject::from_object(
        dict.get("connect")
            .expect("connect function not found")
            .clone(),
    )?;

    let disconnect: Function<hermes::api::DisconnectArgs, ()> = FromObject::from_object(
        dict.get("disconnect")
            .expect("disconnect function not found")
            .clone(),
    )?;

    // Start mock agent for this test
    let (agent, conn_rx) = MockAgent::new();
    let mock_handle = MockAgent::start(agent, conn_rx).expect("Failed to start mock agent");

    let mut options = Dictionary::new();
    options.insert("protocol", "tcp");
    options.insert("host", "localhost");
    options.insert("port", mock_handle.port() as i64);

    connect.call((nvim_oxi::String::from("mock-agent"), Some(options)))?;

    // Wait for initialization before disconnecting
    let wait_for_init = autocommand::listen_for_autocommand::<
        agent_client_protocol::schema::InitializeResponse,
    >(Commands::ConnectionInitialized);
    let _ = wait_for_init(Duration::from_secs(TIMEOUT_IN_SECONDS))?;

    // Disconnect All should succeed
    disconnect.call(hermes::api::DisconnectArgs::All)?;

    // Cleanup
    mock_handle.close();

    Ok(())
}

#[nvim_oxi::test]
fn test_disconnect_single() -> Result<(), nvim_oxi::Error> {
    let dict: Dictionary = hermes::nvim::hermes()?;

    let connect: Function<ConnectionArgs, ()> = FromObject::from_object(
        dict.get("connect")
            .expect("connect function not found")
            .clone(),
    )?;

    let disconnect: Function<hermes::api::DisconnectArgs, ()> = FromObject::from_object(
        dict.get("disconnect")
            .expect("disconnect function not found")
            .clone(),
    )?;

    // Start mock agent for this test
    let (agent, conn_rx) = MockAgent::new();
    let mock_handle = MockAgent::start(agent, conn_rx).expect("Failed to start mock agent");

    let mut options = Dictionary::new();
    options.insert("protocol", "tcp");
    options.insert("host", "localhost");
    options.insert("port", mock_handle.port() as i64);

    connect.call((nvim_oxi::String::from("mock-agent"), Some(options)))?;

    // Wait for initialization before disconnecting
    let wait_for_init = autocommand::listen_for_autocommand::<
        agent_client_protocol::schema::InitializeResponse,
    >(Commands::ConnectionInitialized);
    let _ = wait_for_init(Duration::from_secs(TIMEOUT_IN_SECONDS))?;

    // Disconnect single agent by name
    disconnect.call(hermes::api::DisconnectArgs::Single(Assistant::from(
        "mock-agent",
    )))?;

    // Cleanup
    mock_handle.close();

    Ok(())
}

#[nvim_oxi::test]
fn test_disconnect_multiple() -> Result<(), nvim_oxi::Error> {
    let dict: Dictionary = hermes::nvim::hermes()?;

    let connect: Function<ConnectionArgs, ()> = FromObject::from_object(
        dict.get("connect")
            .expect("connect function not found")
            .clone(),
    )?;

    let disconnect: Function<hermes::api::DisconnectArgs, ()> = FromObject::from_object(
        dict.get("disconnect")
            .expect("disconnect function not found")
            .clone(),
    )?;

    // Start mock agent for this test
    let (agent, conn_rx) = MockAgent::new();
    let mock_handle = MockAgent::start(agent, conn_rx).expect("Failed to start mock agent");

    let mut options = Dictionary::new();
    options.insert("protocol", "tcp");
    options.insert("host", "localhost");
    options.insert("port", mock_handle.port() as i64);

    connect.call((nvim_oxi::String::from("mock-agent"), Some(options)))?;

    // Wait for initialization before disconnecting
    let wait_for_init = autocommand::listen_for_autocommand::<
        agent_client_protocol::schema::InitializeResponse,
    >(Commands::ConnectionInitialized);
    let _ = wait_for_init(Duration::from_secs(TIMEOUT_IN_SECONDS))?;

    // Disconnect multiple agents including the connected one
    disconnect.call(hermes::api::DisconnectArgs::Multiple(vec![
        Assistant::from("mock-agent"),
    ]))?;

    // Cleanup
    mock_handle.close();

    Ok(())
}
