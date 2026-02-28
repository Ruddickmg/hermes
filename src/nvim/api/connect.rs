use std::{rc::Rc, sync::Mutex};

use nvim_oxi::{
    Dictionary, Function,
    lua::{Error, Poppable, Pushable, ffi::State},
};

use crate::{
    apc::connection::{Assistant, ConnectionDetails, Protocol},
    nvim::state::PluginState,
};

#[derive(Clone)]
pub struct ConnectionArgs {
    pub agent: Option<Assistant>,
    pub protocol: Option<Protocol>,
}

impl From<ConnectionArgs> for ConnectionDetails {
    fn from(args: ConnectionArgs) -> Self {
        ConnectionDetails {
            agent: args.agent.unwrap_or_default(),
            protocol: args.protocol.unwrap_or_default(),
        }
    }
}

impl Poppable for ConnectionArgs {
    unsafe fn pop(state: *mut State) -> Result<Self, Error> {
        use nvim_oxi::{Object, ObjectKind};

        let table = unsafe { Dictionary::pop(state)? };

        let agent = table
            .get("agent")
            .map(|v: &Object| {
                if v.kind() != ObjectKind::String {
                    return Err(Error::RuntimeError(
                        "Invalid input for \"agent\", must be a string".to_string(),
                    ));
                }
                let s: nvim_oxi::NvimStr = unsafe { v.as_nvim_str_unchecked() };
                Ok(Assistant::from(s.to_string()))
            })
            .transpose()?;

        let protocol = table
            .get("protocol")
            .map(|v: &Object| {
                if v.kind() != ObjectKind::String {
                    return Err(Error::RuntimeError(
                        "Invalid input for \"protocol\", must be a string".to_string(),
                    ));
                }
                let s: nvim_oxi::NvimStr = unsafe { v.as_nvim_str_unchecked() };
                Ok(Protocol::from(s.to_string()))
            })
            .transpose()?;

        Ok(Self { agent, protocol })
    }
}

impl Pushable for ConnectionArgs {
    unsafe fn push(self, state: *mut State) -> Result<i32, Error> {
        let dict = nvim_oxi::Object::from({
            let mut dict = Dictionary::new();

            if let Some(agent) = self.agent {
                dict.insert("agent", agent.to_string());
            }

            if let Some(protocol) = self.protocol {
                dict.insert("protocol", protocol.to_string());
            }

            dict
        });

        // SAFETY: Caller must ensure valid state pointer
        unsafe { dict.push(state) }
    }
}

pub fn create_lua_connect(
    plugin_state: Rc<Mutex<PluginState>>,
) -> Function<Option<ConnectionArgs>, Result<(), Error>> {
    Function::from_fn(move |arg: Option<ConnectionArgs>| {
        let details = arg.map(ConnectionDetails::from).unwrap_or_default();
        plugin_state
            .lock()
            .map_err(|e| Error::RuntimeError(e.to_string()))?
            .connection
            .connect(details.clone())
            .map_err(Error::from)?;
        Ok(())
    })
}
