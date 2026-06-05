//! Integration tests for nvim module components

use hermes::nvim::hermes;
use nvim_oxi::Dictionary;

pub mod api;
pub mod setup;
pub mod terminal;

#[nvim_oxi::test]
fn hermes_initializes_without_error() -> nvim_oxi::Result<()> {
    let dict: Dictionary = hermes()?;
    assert!(
        dict.get("setup").is_some(),
        "hermes dictionary should contain 'setup' export"
    );
    Ok(())
}
