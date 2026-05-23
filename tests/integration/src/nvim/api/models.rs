use crate::helpers::mock_runtime;
use async_lock::Mutex;
use hermes::{
    Handler, PluginState, api::Api, nvim::requests::Requests,
    utilities::detect_project_storage_path,
};
use nvim_oxi::{Array, Dictionary, Object};
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
fn models_returns_session_not_found_when_no_session_info() -> nvim_oxi::Result<()> {
    let plugin_state = Arc::new(Mutex::new(PluginState::new()));
    let logger =
        hermes::utilities::logging::Logger::inititalize(&detect_project_storage_path().unwrap())
            .unwrap();
    let api = create_test_api(plugin_state, logger);

    let result = block_on(api.models("nonexistent-session".to_string()));

    assert!(matches!(
        result,
        Err(hermes::acp::error::Error::SessionNotFound(_))
    ));

    Ok(())
}

#[nvim_oxi::test]
fn models_returns_unsupported_when_session_has_no_model_info() -> nvim_oxi::Result<()> {
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

    let result = block_on(api.models("test-session".to_string()));

    assert!(matches!(
        result,
        Err(hermes::acp::error::Error::Unsupported(_))
    ));

    Ok(())
}

#[nvim_oxi::test]
fn models_returns_legacy_models() -> nvim_oxi::Result<()> {
    let plugin_state = Arc::new(Mutex::new(PluginState::new()));
    let logger =
        hermes::utilities::logging::Logger::inititalize(&detect_project_storage_path().unwrap())
            .unwrap();
    let api = create_test_api(plugin_state.clone(), logger);

    {
        let mut state = block_on(plugin_state.lock());
        let model = agent_client_protocol::schema::ModelInfo::new("gpt4", "GPT-4");
        let models = agent_client_protocol::schema::SessionModelState::new("gpt4", vec![model]);
        let session =
            agent_client_protocol::schema::NewSessionResponse::new("test-session").models(models);
        state.set_session_info(&session);
    }

    let result = block_on(api.models("test-session".to_string()));

    let mut expected = Array::new();
    let mut dict = Dictionary::new();
    dict.insert("value", "gpt4");
    dict.insert("name", "GPT-4");
    expected.push(Object::from(dict));
    assert_eq!(result.unwrap(), expected);

    Ok(())
}

#[nvim_oxi::test]
fn models_returns_config_options() -> nvim_oxi::Result<()> {
    let plugin_state = Arc::new(Mutex::new(PluginState::new()));
    let logger =
        hermes::utilities::logging::Logger::inititalize(&detect_project_storage_path().unwrap())
            .unwrap();
    let api = create_test_api(plugin_state.clone(), logger);

    {
        let mut state = block_on(plugin_state.lock());
        let option = agent_client_protocol::schema::SessionConfigOption::select(
            "model",
            "Model",
            "gpt4",
            vec![agent_client_protocol::schema::SessionConfigSelectOption::new("gpt4", "GPT-4")],
        )
        .category(agent_client_protocol::schema::SessionConfigOptionCategory::Model);
        let session = agent_client_protocol::schema::NewSessionResponse::new("test-session")
            .config_options(vec![option]);
        state.set_session_info(&session);
    }

    let result = block_on(api.models("test-session".to_string()));

    let mut expected = Array::new();
    let mut dict = Dictionary::new();
    dict.insert("value", "gpt4");
    dict.insert("name", "GPT-4");
    expected.push(Object::from(dict));
    assert_eq!(result.unwrap(), expected);

    Ok(())
}
