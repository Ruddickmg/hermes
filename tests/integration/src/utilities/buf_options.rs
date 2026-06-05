//! Integration tests for buffer option utilities
use hermes::utilities::buf_options::{buf_get_name, get_buf_option, set_buf_option};
use pretty_assertions::assert_eq;

#[nvim_oxi::test]
fn set_and_get_buf_option_roundtrip() -> nvim_oxi::Result<()> {
    let buf = nvim_oxi::api::create_buf(false, true)?;
    set_buf_option("swapfile", true, &buf)?;
    let swapfile: bool = get_buf_option("swapfile", &buf)?;
    assert!(swapfile, "swapfile should be true after setting");
    Ok(())
}

#[nvim_oxi::test]
fn set_buf_option_overwrites_existing_value() -> nvim_oxi::Result<()> {
    let buf = nvim_oxi::api::create_buf(false, true)?;
    set_buf_option("swapfile", true, &buf)?;
    set_buf_option("swapfile", false, &buf)?;
    let swapfile: bool = get_buf_option("swapfile", &buf)?;
    assert!(!swapfile, "swapfile should be false after overwriting");
    Ok(())
}

#[nvim_oxi::test]
fn buf_get_name_returns_empty_for_unnamed() -> nvim_oxi::Result<()> {
    let buf = nvim_oxi::api::create_buf(false, true)?;
    let name = buf_get_name(&buf)?;
    assert!(name.is_empty(), "unnamed buffer should have empty name");
    Ok(())
}

#[nvim_oxi::test]
fn buf_get_name_returns_path_for_named() -> nvim_oxi::Result<()> {
    let path = "/tmp/hermes_test_buf.txt";
    nvim_oxi::api::command(&format!("edit {}", path))?;
    let buf = nvim_oxi::api::get_current_buf();
    let name = buf_get_name(&buf)?;
    assert_eq!(name, path, "named buffer should return its path");
    Ok(())
}

#[nvim_oxi::test]
fn get_win_option_returns_number_for_current_window() -> nvim_oxi::Result<()> {
    use hermes::utilities::buf_options::get_win_option;

    let win = nvim_oxi::api::get_current_win();
    let number: bool = get_win_option("number", &win)?;
    assert_eq!(
        number, false,
        "current window number option should be false by default in test"
    );
    Ok(())
}

#[nvim_oxi::test]
fn get_buf_option_errors_for_invalid_option() -> nvim_oxi::Result<()> {
    let buf = nvim_oxi::api::create_buf(false, true)?;
    let result = get_buf_option::<bool>("not_a_real_option", &buf);
    assert!(
        result.is_err(),
        "invalid option name should return an error"
    );
    Ok(())
}

#[nvim_oxi::test]
fn set_buf_option_errors_for_invalid_option() -> nvim_oxi::Result<()> {
    let buf = nvim_oxi::api::create_buf(false, true)?;
    let result = set_buf_option("not_a_real_option", true, &buf);
    assert!(
        result.is_err(),
        "invalid option name should return an error"
    );
    Ok(())
}
