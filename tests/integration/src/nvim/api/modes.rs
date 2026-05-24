use crate::helpers::mock_runtime;
use async_lock::Mutex;
use hermes::{
    Handler, PluginState,
    api::{Api, ModesArgs},
    nvim::requests::Requests,
    utilities::detect_project_storage_path,
};
use nvim_oxi::Object;
use std::rc::Rc;
use std::sync::Arc;

fn create_test_api(
    plugin_state: Arc<Mutex<PluginState>>,
    logger: &'static hermes::utilities::Logger,
) -> hermes::api::Api {
    let runtime = mock_runtime();
    let requests = Rc::new(
        Requests::new(runtime.clone(), plugin_state.clone()).expect("Failed to create requests"),
    );
    let handler = Arc::new(
        Handler::new(plugin_state.clone(), runtime.clone(), requests.clone())
            .expect("Failed to create handler"),
    );
    Api::new(plugin_state, logger, handler, requests)
}

fn block_on<F>(fut: F) -> F::Output
where
    F: std::future::Future,
{
    futures::executor::block_on(fut)
}

#[nvim_oxi::test]
fn modes_returns_session_not_found_when_no_session_info() -> nvim_oxi::Result<()> {
    let plugin_state = Arc::new(Mutex::new(PluginState::new()));
    let logger =
        hermes::utilities::logging::Logger::inititalize(&detect_project_storage_path().unwrap())
            .unwrap();
    let api = create_test_api(plugin_state, logger);

    let result = block_on(api.modes("nonexistent-session".to_string()));

    assert!(matches!(
        result,
        Err(hermes::acp::error::Error::SessionNotFound(_))
    ));

    Ok(())
}

#[nvim_oxi::test]
fn modes_returns_unsupported_when_session_has_no_mode_info() -> nvim_oxi::Result<()> {
    let plugin_state = Arc::new(Mutex::new(PluginState::new()));
    let logger =
        hermes::utilities::logging::Logger::inititalize(&detect_project_storage_path().unwrap())
            .unwrap();
    let api = create_test_api(plugin_state.clone(), logger);

    {
        let mut state = block_on(plugin_state.lock());
        let session = agent_client_protocol::schema::NewSessionResponse::new("test-session");
        state.set_session_info(&session);
    }

    let result = block_on(api.modes("test-session".to_string()));

    assert!(matches!(
        result,
        Err(hermes::acp::error::Error::Unsupported(_))
    ));

    Ok(())
}

#[nvim_oxi::test]
fn modes_returns_legacy_modes() -> nvim_oxi::Result<()> {
    let plugin_state = Arc::new(Mutex::new(PluginState::new()));
    let logger =
        hermes::utilities::logging::Logger::inititalize(&detect_project_storage_path().unwrap())
            .unwrap();
    let api = create_test_api(plugin_state.clone(), logger);

    {
        let mut state = block_on(plugin_state.lock());
        let mode = agent_client_protocol::schema::SessionMode::new("chat", "Chat");
        let modes = agent_client_protocol::schema::SessionModeState::new("chat", vec![mode]);
        let session =
            agent_client_protocol::schema::NewSessionResponse::new("test-session").modes(modes);
        state.set_session_info(&session);
    }

    let result = block_on(api.modes("test-session".to_string()));

    assert!(result.is_ok(), "modes should succeed for legacy session");
    let array = result.unwrap();
    assert_eq!(array.len(), 1, "Should return one mode");

    Ok(())
}

#[nvim_oxi::test]
fn modes_returns_config_options() -> nvim_oxi::Result<()> {
    let plugin_state = Arc::new(Mutex::new(PluginState::new()));
    let logger =
        hermes::utilities::logging::Logger::inititalize(&detect_project_storage_path().unwrap())
            .unwrap();
    let api = create_test_api(plugin_state.clone(), logger);

    {
        let mut state = block_on(plugin_state.lock());
        let option = agent_client_protocol::schema::SessionConfigOption::select(
            "mode",
            "Mode",
            "chat",
            vec![agent_client_protocol::schema::SessionConfigSelectOption::new("chat", "Chat")],
        )
        .category(agent_client_protocol::schema::SessionConfigOptionCategory::Mode);
        let session = agent_client_protocol::schema::NewSessionResponse::new("test-session")
            .config_options(vec![option]);
        state.set_session_info(&session);
    }

    let result = block_on(api.modes("test-session".to_string()));

    assert!(
        result.is_ok(),
        "modes should succeed for config options session"
    );
    let array = result.unwrap();
    assert_eq!(array.len(), 1, "Should return one mode");

    Ok(())
}

#[nvim_oxi::test]
fn modes_returns_grouped_options_with_group() -> nvim_oxi::Result<()> {
    let plugin_state = Arc::new(Mutex::new(PluginState::new()));
    let logger =
        hermes::utilities::logging::Logger::inititalize(&detect_project_storage_path().unwrap())
            .unwrap();
    let api = create_test_api(plugin_state.clone(), logger);

    {
        let mut state = block_on(plugin_state.lock());
        let group = agent_client_protocol::schema::SessionConfigSelectGroup::new(
            "my-group",
            "My Group",
            vec![agent_client_protocol::schema::SessionConfigSelectOption::new("chat", "Chat")],
        );
        let option = agent_client_protocol::schema::SessionConfigOption::new(
            "mode",
            "Mode",
            agent_client_protocol::schema::SessionConfigKind::Select(
                agent_client_protocol::schema::SessionConfigSelect::new("chat", vec![group]),
            ),
        )
        .category(agent_client_protocol::schema::SessionConfigOptionCategory::Mode);
        let session = agent_client_protocol::schema::NewSessionResponse::new("test-session")
            .config_options(vec![option]);
        state.set_session_info(&session);
    }

    let result = block_on(api.modes("test-session".to_string()));

    assert!(
        result.is_ok(),
        "modes should succeed for grouped options session"
    );
    let array = result.unwrap();
    assert_eq!(array.len(), 1, "Should return one mode");

    let obj = array.get(0).expect("Array should have one element");
    let dict: nvim_oxi::Dictionary = obj.clone().try_into().expect("Should be a dictionary");
    let group_value: nvim_oxi::String = dict
        .get("group")
        .expect("Should have group key")
        .clone()
        .try_into()
        .expect("Should be a string");
    assert_eq!(group_value.to_string(), "My Group");

    Ok(())
}
