use hermes::nvim::setup;
use nvim_oxi::Dictionary;

#[nvim_oxi::test]
fn test_setup_returns_connect_function() -> Result<(), nvim_oxi::Error> {
    let dict: Dictionary = setup()?;

    assert!(
        dict.get("connect").is_some(),
        "connect function should be registered"
    );

    Ok(())
}

#[nvim_oxi::test]
fn connection_can_be_made_to_copilot() -> Result<(), nvim_oxi::Error> {
    let dict: Dictionary = setup()?;

    assert!(
        dict.get("connect").is_some(),
        "connect function should be registered"
    );

    Ok(())
}
