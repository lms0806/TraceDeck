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

#[cfg(test)]
mod tests {
    use super::ConnectionProfile;

    #[test]
    fn deserializes_legacy_profiles_and_serializes_camel_case_fields() {
        let profile: ConnectionProfile = serde_json::from_str(
            r#"{
                "id":"prod",
                "name":"Production",
                "host":"prod.example.test",
                "port":22,
                "username":"deploy",
                "group":"Production"
            }"#,
        )
        .unwrap();
        assert_eq!(profile.auth_kind, "privateKey");
        assert!(profile.private_key_path.is_none());

        let json = serde_json::to_value(profile).unwrap();
        assert_eq!(json["authKind"], "privateKey");
        assert!(json.get("privateKeyPath").is_some());
    }
}
