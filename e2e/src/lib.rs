use hermes::nvim::{setup, ConnectionArgs};
use nvim_oxi::{conversion::FromObject, Dictionary, Function};

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
fn test_connect_function_signature() -> Result<(), nvim_oxi::Error> {
    let dict: Dictionary = setup()?;

    let connect_obj = dict.get("connect").expect("connect function not found");
    let _connect: Function<Option<ConnectionArgs>, ()> =
        FromObject::from_object(connect_obj.clone())?;

    Ok(())
}
