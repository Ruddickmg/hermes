use futures::future;
use nvim_oxi::{
    Object, ObjectKind,
    conversion::FromObject,
    lua::{self, Poppable, Pushable},
};
use tracing::error;

use agent_client_protocol::schema::{DeleteSessionRequest, SessionId};

use crate::{
    acp::{self, error::Error},
    api::Api,
};

#[derive(Clone, Debug)]
pub enum DeleteSessionArgs {
    Multiple(Vec<String>),
    Single(String),
}

const EXPECTED: &str = "String or Array of Strings";

impl FromObject for DeleteSessionArgs {
    fn from_object(obj: Object) -> Result<Self, nvim_oxi::conversion::Error> {
        match obj.kind() {
            ObjectKind::String => {
                let session_id = unsafe { obj.into_string_unchecked() };
                Ok(Self::Single(session_id.to_string()))
            }
            ObjectKind::Array => {
                let sessions = unsafe { obj.into_array_unchecked() };
                Ok(Self::Multiple(
                    sessions
                        .into_iter()
                        .map(|obj| {
                            if let ObjectKind::String = obj.kind() {
                                Ok(unsafe { obj.into_string_unchecked() })
                            } else {
                                Err(nvim_oxi::conversion::Error::FromWrongType {
                                    expected: EXPECTED,
                                    actual: obj.kind().as_static(),
                                })
                            }
                        })
                        .collect::<Result<Vec<nvim_oxi::String>, nvim_oxi::conversion::Error>>()?
                        .into_iter()
                        .map(|s| s.to_string())
                        .collect(),
                ))
            }
            other => Err(nvim_oxi::conversion::Error::FromWrongType {
                expected: EXPECTED,
                actual: other.as_static(),
            }),
        }
    }
}

impl Poppable for DeleteSessionArgs {
    unsafe fn pop(lua: *mut nvim_oxi::lua::ffi::State) -> Result<Self, lua::Error> {
        let obj = unsafe { Object::pop(lua)? };
        Ok(Self::from_object(obj)
            .inspect_err(|e| {
                error!(
                    "An error occurred while parsing the delete_session arguments: {:?}",
                    e
                )
            })
            .unwrap_or(Self::Multiple(vec![])))
    }
}

impl Pushable for DeleteSessionArgs {
    unsafe fn push(self, state: *mut lua::ffi::State) -> Result<i32, lua::Error> {
        unsafe {
            match self {
                Self::Single(s) => s.push(state),
                Self::Multiple(vec) => vec.push(state),
            }
        }
    }
}

impl Api {
    #[tracing::instrument(level = "trace", skip(self))]
    pub async fn delete_session(&self, args: DeleteSessionArgs) -> acp::Result<()> {
        let state = self.state.lock().await;
        let agent_info = state.agent_info.clone();
        drop(state);

        if !agent_info.can_delete_session() {
            return Ok(());
        }

        let session_ids: Vec<String> = match args {
            DeleteSessionArgs::Single(id) => vec![id],
            DeleteSessionArgs::Multiple(ids) => ids,
        };

        let connection = self
            .connection
            .get_current_connection()
            .await
            .ok_or_else(|| Error::Connection("No connection found".to_string()))?;

        let futures = session_ids
            .into_iter()
            .map(|session_id| {
                connection.delete_session(DeleteSessionRequest::new(SessionId::from(session_id)))
            })
            .collect::<Vec<_>>();

        future::try_join_all(futures).await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    fn arb_session_id() -> impl Strategy<Value = String> {
        prop_oneof![
            Just("test-session".to_string()),
            "[a-zA-Z0-9_-]{1,20}".prop_map(|s| s.to_string())
        ]
    }

    fn arb_delete_session_args() -> impl Strategy<Value = DeleteSessionArgs> {
        prop_oneof![
            arb_session_id().prop_map(DeleteSessionArgs::Single),
            prop::collection::vec(arb_session_id(), 0..5).prop_map(DeleteSessionArgs::Multiple),
        ]
    }

    proptest! {
        #[test]
        fn test_delete_session_args_roundtrip(args in arb_delete_session_args()) {
            match args {
                DeleteSessionArgs::Single(ref id) => {
                    prop_assert!(!id.is_empty(), "Single session ID should not be empty");
                }
                DeleteSessionArgs::Multiple(ref ids) => {
                    prop_assert!(ids.len() <= 5, "Multiple should have at most 5 session IDs");
                }
            }
        }
    }
}
