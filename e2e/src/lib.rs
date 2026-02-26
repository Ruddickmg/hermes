use hermes::{
    apc::connection::{Assistant, Protocol},
    nvim::{ConnectionArgs, setup},
};
use nvim_oxi::{Dictionary, Function, conversion::FromObject};

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
async fn test_connect_function() -> Result<(), nvim_oxi::Error> {
    let dict: Dictionary = setup()?;

    let connect_obj = dict.get("connect").expect("connect function not found");
    let connect: Function<Option<ConnectionArgs>, ()> =
        FromObject::from_object(connect_obj.clone())?;

    connect.call(Some(ConnectionArgs {
        agent: Some(Assistant::Opencode),
        protocol: Some(Protocol::Stdio),
    }))?;

    Ok(())
}
