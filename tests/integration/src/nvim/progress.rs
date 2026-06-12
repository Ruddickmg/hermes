//! Integration tests for progress configuration
//!
//! These tests verify that show_progress_in_cmdline() correctly modifies
//! Neovim's messagesopt option.

use hermes::nvim::configuration::show_progress_in_cmdline;
use nvim_oxi;

/// Helper to read the current messagesopt value
fn get_messagesopt() -> String {
    nvim_oxi::api::call_function::<(String,), String>("execute", ("set messagesopt?".to_string(),))
        .unwrap_or_default()
}

#[nvim_oxi::test]
fn show_progress_in_cmdline_enables_progress() -> nvim_oxi::Result<()> {
    show_progress_in_cmdline(true);

    let messagesopt = get_messagesopt();
    assert!(
        messagesopt.contains("progress:c"),
        "messagesopt should contain progress:c when enabled, got: {}",
        messagesopt
    );
    Ok(())
}

#[nvim_oxi::test]
fn show_progress_in_cmdline_disables_progress() -> nvim_oxi::Result<()> {
    show_progress_in_cmdline(true);
    show_progress_in_cmdline(false);

    let messagesopt = get_messagesopt();
    assert!(
        !messagesopt.contains("progress:c"),
        "messagesopt should not contain progress:c when disabled, got: {}",
        messagesopt
    );
    Ok(())
}
