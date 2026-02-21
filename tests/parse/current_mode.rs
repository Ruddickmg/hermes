use agent_client_protocol::{CurrentModeUpdate, SessionModeId};
use hermes::nvim::event::current_mode_event;

#[test]
fn test_current_mode_event_ok() {
    let update = CurrentModeUpdate::new(SessionModeId::new("ask"));

    let result = current_mode_event(update);
    assert_eq!(result.is_ok(), true);
}

#[test]
fn test_current_mode_event_id_ask() {
    let update = CurrentModeUpdate::new(SessionModeId::new("ask"));

    let result = current_mode_event(update).unwrap();
    let id = result.get("id").unwrap();
    assert_eq!(*id, nvim_oxi::Object::from("ask"));
}

#[test]
fn test_current_mode_event_without_meta() {
    let update = CurrentModeUpdate::new(SessionModeId::new("ask"));

    let result = current_mode_event(update).unwrap();
    assert_eq!(result.get("meta").is_some(), false);
}

#[test]
fn test_current_mode_event_with_meta() {
    let meta: serde_json::Map<String, serde_json::Value> = serde_json::json!({"source": "user"})
        .as_object()
        .unwrap()
        .clone();
    let update = CurrentModeUpdate::new(SessionModeId::new("code")).meta(meta);

    let result = current_mode_event(update).unwrap();
    assert_eq!(result.get("meta").is_some(), true);
}
