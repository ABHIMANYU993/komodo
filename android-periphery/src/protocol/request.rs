use serde::{Deserialize, Serialize};
use uuid::Uuid;
use super::types::PollStatus;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct RawPeripheryRequest {
    #[serde(rename = "type")]
    pub request_type: String,
    #[serde(default)]
    pub params: serde_json::Value,
}

#[derive(Debug, Clone)]
pub enum PeripheryRequest {
    PollStatus(PollStatus),
    GetHealth,
    GetVersion,
    GetSystemProcesses,

    // Terminal requests
    ListTerminals {
        target: Option<String>,
    },
    CreateServerTerminal {
        name: Option<String>,
        command: Option<String>,
        recreate: Option<String>,
    },
    ConnectTerminal {
        terminal: String,
        target: String,
    },
    DisconnectTerminal {
        channel: Uuid,
    },
    DeleteTerminal {
        terminal: String,
        target: String,
    },
    ExecuteTerminal {
        terminal: String,
        target: String,
        command: String,
    },

    // Any request not supported on Android (e.g. Docker, Swarm, Compose)
    Unsupported(String),
}

impl PeripheryRequest {
    pub fn parse(json_bytes: &[u8]) -> anyhow::Result<Self> {
        let raw: RawPeripheryRequest = serde_json::from_slice(json_bytes)?;
        match raw.request_type.as_str() {
            "PollStatus" => {
                let params: PollStatus = serde_json::from_value(raw.params)?;
                Ok(PeripheryRequest::PollStatus(params))
            }
            "GetHealth" => Ok(PeripheryRequest::GetHealth),
            "GetVersion" => Ok(PeripheryRequest::GetVersion),
            "GetSystemProcesses" => Ok(PeripheryRequest::GetSystemProcesses),
            "ListTerminals" => {
                let target = raw.params.get("target").and_then(|v| v.as_str()).map(str::to_string);
                Ok(PeripheryRequest::ListTerminals { target })
            }
            "CreateServerTerminal" => {
                let name = raw.params.get("name").and_then(|v| v.as_str()).map(str::to_string);
                let command = raw.params.get("command").and_then(|v| v.as_str()).map(str::to_string);
                let recreate = raw.params.get("recreate").and_then(|v| v.as_str()).map(str::to_string);
                Ok(PeripheryRequest::CreateServerTerminal { name, command, recreate })
            }
            "ConnectTerminal" => {
                let terminal = raw.params.get("terminal").and_then(|v| v.as_str()).unwrap_or_default().to_string();
                let target = raw.params.get("target").and_then(|v| v.as_str()).unwrap_or_default().to_string();
                Ok(PeripheryRequest::ConnectTerminal { terminal, target })
            }
            "DisconnectTerminal" => {
                let channel: Uuid = serde_json::from_value(raw.params.get("channel").cloned().unwrap_or_default())?;
                Ok(PeripheryRequest::DisconnectTerminal { channel })
            }
            "DeleteTerminal" => {
                let terminal = raw.params.get("terminal").and_then(|v| v.as_str()).unwrap_or_default().to_string();
                let target = raw.params.get("target").and_then(|v| v.as_str()).unwrap_or_default().to_string();
                Ok(PeripheryRequest::DeleteTerminal { terminal, target })
            }
            "ExecuteTerminal" => {
                let terminal = raw.params.get("terminal").and_then(|v| v.as_str()).unwrap_or_default().to_string();
                let target = raw.params.get("target").and_then(|v| v.as_str()).unwrap_or_default().to_string();
                let command = raw.params.get("command").and_then(|v| v.as_str()).unwrap_or_default().to_string();
                Ok(PeripheryRequest::ExecuteTerminal { terminal, target, command })
            }
            other => Ok(PeripheryRequest::Unsupported(other.to_string())),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_deserialize_poll_status() {
        let json = br#"{"type":"PollStatus","params":{"include_stats":true,"include_docker":false}}"#;
        let req = PeripheryRequest::parse(json).unwrap();
        match req {
            PeripheryRequest::PollStatus(p) => {
                assert!(p.include_stats);
                assert!(!p.include_docker);
            }
            _ => panic!("Expected PollStatus"),
        }
    }

    #[test]
    fn test_deserialize_get_health() {
        let json = br#"{"type":"GetHealth","params":{}}"#;
        let req = PeripheryRequest::parse(json).unwrap();
        assert!(matches!(req, PeripheryRequest::GetHealth));
    }

    #[test]
    fn test_deserialize_get_version() {
        let json = br#"{"type":"GetVersion","params":{}}"#;
        let req = PeripheryRequest::parse(json).unwrap();
        assert!(matches!(req, PeripheryRequest::GetVersion));
    }

    #[test]
    fn test_deserialize_unsupported() {
        let json = br#"{"type":"RunContainer","params":{"image":"alpine"}}"#;
        let req = PeripheryRequest::parse(json).unwrap();
        match req {
            PeripheryRequest::Unsupported(op) => assert_eq!(op, "RunContainer"),
            _ => panic!("Expected Unsupported"),
        }
    }
}
