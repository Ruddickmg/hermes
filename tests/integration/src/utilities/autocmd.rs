//! Integration tests for autocommand utilities
use hermes::utilities::autocmd::{
    autocmd_listeners_attached, create_augroup, create_autocmd, exec_autocmd,
};
use std::cell::RefCell;
use std::rc::Rc;

#[nvim_oxi::test]
fn create_augroup_returns_positive_id() -> nvim_oxi::Result<()> {
    let id = create_augroup("hermes_test_group", true)?;
    assert!(id > 0, "augroup id should be positive");
    Ok(())
}

#[nvim_oxi::test]
fn create_autocmd_and_exec_autocmd_trigger_callback() -> nvim_oxi::Result<()> {
    let group_name = "hermes_test_exec";
    let group_id = create_augroup(group_name, true)?;

    let triggered = Rc::new(RefCell::new(false));
    let triggered_clone = triggered.clone();
    create_autocmd(group_id, "User", move || {
        *triggered_clone.borrow_mut() = true;
        Ok(true)
    })?;

    exec_autocmd(group_name, "User", "*", nvim_oxi::Object::from("test_data"))?;

    assert!(
        triggered.borrow().clone(),
        "callback should have been triggered by exec_autocmd"
    );
    Ok(())
}

#[nvim_oxi::test]
fn autocmd_listeners_attached_true_when_listener_exists() -> nvim_oxi::Result<()> {
    let group_name = "hermes_test_attached";
    let group_id = create_augroup(group_name, true)?;

    create_autocmd(group_id, "User", || Ok(true))?;

    assert!(
        autocmd_listeners_attached(group_name, "User", "*"),
        "should return true when a listener is registered"
    );
    Ok(())
}

#[nvim_oxi::test]
fn autocmd_listeners_attached_false_when_no_listener() -> nvim_oxi::Result<()> {
    let group_name = "hermes_test_not_attached";
    create_augroup(group_name, true)?;

    assert!(
        !autocmd_listeners_attached(group_name, "User", "MissingPattern"),
        "should return false when no listener is registered"
    );
    Ok(())
}

#[nvim_oxi::test]
fn create_autocmd_rejects_empty_event() -> nvim_oxi::Result<()> {
    let group_id = create_augroup("hermes_test_empty_event", true)?;
    let result = create_autocmd(group_id, "", || Ok(true));
    assert!(result.is_err(), "empty event should fail");
    Ok(())
}

#[nvim_oxi::test]
fn exec_autocmd_errors_for_nonexistent_group() -> nvim_oxi::Result<()> {
    let result = exec_autocmd(
        "nonexistent_group_xyz",
        "User",
        "*",
        nvim_oxi::Object::from(()),
    );
    assert!(result.is_err(), "nonexistent group should produce an error");
    Ok(())
}

#[nvim_oxi::test]
fn create_autocmd_callback_error_path_logs_and_returns_nil() -> nvim_oxi::Result<()> {
    let group_id = create_augroup("hermes_test_cb_err", true)?;

    create_autocmd(group_id, "User", || {
        Err(hermes::acp::error::Error::Internal(
            "test error".to_string(),
        ))
    })?;

    // Trigger the autocmd; the callback should execute and return nil internally.
    exec_autocmd(
        "hermes_test_cb_err",
        "User",
        "*",
        nvim_oxi::Object::from(()),
    )?;

    Ok(())
}
