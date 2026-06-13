//! Cross-process control protocol for the DayRecord capture service.

use crate::models::DayStats;
use serde::{Deserialize, Serialize};
use schemars::JsonSchema;

pub const CONTROL_SOCKET_NAME: &str = "dayrecord-control";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "cmd", rename_all = "snake_case")]
pub enum ControlCommand {
    Pause,
    Resume,
    Status,
    GenerateSummary {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        day: Option<String>,
    },
    Consolidate {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        day: Option<String>,
    },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ControlResponse {
    pub ok: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub data: Option<ControlData>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct ControlData {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub recording: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub day: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stats: Option<DayStats>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub summary_markdown: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fact_count: Option<usize>,
}

impl ControlResponse {
    pub fn ok(data: ControlData) -> Self {
        Self {
            ok: true,
            error: None,
            data: Some(data),
        }
    }

    pub fn err(message: impl Into<String>) -> Self {
        Self {
            ok: false,
            error: Some(message.into()),
            data: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ControlError {
    ServiceNotRunning,
    /// MCP daemon autostart blocked (consent off or `mcp_autostart_daemon=false`).
    AutostartDenied(String),
    Transport(String),
    Protocol(String),
}

impl std::fmt::Display for ControlError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ServiceNotRunning => write!(
                f,
                "DayRecord capture service is not running (control IPC offline; \
                 `recording` in settings may still be true)"
            ),
            Self::AutostartDenied(msg) => write!(f, "{msg}"),
            Self::Transport(msg) => write!(f, "control transport error: {msg}"),
            Self::Protocol(msg) => write!(f, "control protocol error: {msg}"),
        }
    }
}

impl std::error::Error for ControlError {}

pub trait ControlClient: Send + Sync {
    fn request(&self, cmd: ControlCommand) -> Result<ControlResponse, ControlError>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn command_roundtrip() {
        let cmd = ControlCommand::GenerateSummary {
            day: Some("2026-06-13".into()),
        };
        let json = serde_json::to_string(&cmd).unwrap();
        let back: ControlCommand = serde_json::from_str(&json).unwrap();
        assert_eq!(cmd, back);
    }

    #[test]
    fn response_roundtrip() {
        let resp = ControlResponse::ok(ControlData {
            recording: Some(true),
            day: Some("2026-06-13".into()),
            stats: Some(DayStats {
                active_seconds: 60,
                session_count: 1,
                char_count: 10,
                pending_chars: 0,
            }),
            summary_markdown: None,
            fact_count: None,
        });
        let json = serde_json::to_string(&resp).unwrap();
        let back: ControlResponse = serde_json::from_str(&json).unwrap();
        assert_eq!(resp.ok, back.ok);
        assert_eq!(resp.data.as_ref().and_then(|d| d.recording), back.data.as_ref().and_then(|d| d.recording));
    }
}
