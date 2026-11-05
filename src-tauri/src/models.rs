use serde::Serialize;

pub(crate) use trace_deck_core::{ConnectionProfile, LogSource, RemoteEntry, RemoteFile};

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ConnectionCheck {
    pub(crate) connected: bool,
    pub(crate) message: String,
}
