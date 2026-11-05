use serde::Serialize;

pub(crate) use trace_deck_core::{ConnectionProfile, LogSource, RemoteEntry, RemoteFile};

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ConnectionCheck {
    pub(crate) connected: bool,
    pub(crate) message: String,
}

#[derive(Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct TerminalChunk {
    pub(crate) terminal_id: u64,
    pub(crate) chunk: String,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RemoteSystemStatus {
    pub(crate) cpu_percent: f64,
    pub(crate) disk_used_kb: u64,
    pub(crate) disk_total_kb: u64,
    pub(crate) disk_percent: f64,
}

#[derive(Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SystemStatusChunk {
    pub(crate) stream_id: u64,
    pub(crate) status: RemoteSystemStatus,
}
