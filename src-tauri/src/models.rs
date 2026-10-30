use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ConnectionProfile {
    pub(crate) id: String,
    pub(crate) name: String,
    pub(crate) host: String,
    pub(crate) port: u16,
    pub(crate) username: String,
    pub(crate) group: String,
    #[serde(default = "default_auth_kind")]
    pub(crate) auth_kind: String,
    #[serde(default)]
    pub(crate) private_key_path: Option<String>,
}

fn default_auth_kind() -> String {
    "privateKey".to_string()
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ConnectionCheck {
    pub(crate) connected: bool,
    pub(crate) message: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RemoteEntry {
    pub(crate) name: String,
    pub(crate) path: String,
    pub(crate) is_dir: bool,
    pub(crate) details: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RemoteFile {
    pub(crate) path: String,
    pub(crate) content: String,
    pub(crate) truncated: bool,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct LogSource {
    pub(crate) path: String,
    pub(crate) content: String,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub(crate) struct TailChunk {
    pub(crate) stream_id: u64,
    pub(crate) chunk: String,
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
