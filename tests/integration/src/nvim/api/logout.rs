use crate::helpers::mock_runtime;
use async_lock::Mutex;
use hermes::{
    Handler, PluginState, api::Api, nvim::requests::Requests,
    utilities::detect_project_storage_path,
};
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
fn logout_all_returns_ok_when_no_connections_exist() -> nvim_oxi::Result<()> {
    let plugin_state = Arc::new(Mutex::new(PluginState::new()));
    let logger =
        hermes::utilities::logging::Logger::inititalize(&detect_project_storage_path().unwrap())
            .unwrap();
    let api = create_test_api(plugin_state, logger);

    let result = block_on(api.logout(hermes::api::LogoutArgs::All));

    assert!(result.is_ok());

    Ok(())
}

#[nvim_oxi::test]
fn logout_multiple_returns_error_when_agents_not_connected() -> nvim_oxi::Result<()> {
    let plugin_state = Arc::new(Mutex::new(PluginState::new()));
    let logger =
        hermes::utilities::logging::Logger::inititalize(&detect_project_storage_path().unwrap())
            .unwrap();
    let api = create_test_api(plugin_state, logger);

    let result = block_on(api.logout(hermes::api::LogoutArgs::Multiple(vec![
        hermes::acp::connection::Assistant::Copilot,
        hermes::acp::connection::Assistant::Opencode,
    ])));

    assert!(matches!(
        result,
        Err(hermes::acp::error::Error::Connection(_))
    ));

    Ok(())
}
