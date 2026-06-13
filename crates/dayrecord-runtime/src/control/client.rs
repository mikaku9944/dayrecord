//! Control IPC clients.

use dayrecord_core::control::{
    ControlClient, ControlCommand, ControlError, ControlResponse,
};
use dayrecord_core::paths;
use std::io::{BufRead, BufReader, Write};
use std::net::TcpStream;
use std::sync::Arc;

pub struct IpcControlClient;

impl ControlClient for IpcControlClient {
    fn request(&self, cmd: ControlCommand) -> Result<ControlResponse, ControlError> {
        ipc_request(&cmd)
    }
}

pub fn ipc_request(cmd: &ControlCommand) -> Result<ControlResponse, ControlError> {
    if !capture_service_likely_running() {
        return Err(ControlError::ServiceNotRunning);
    }
    let port = super::server::read_control_port().map_err(ControlError::Transport)?;
    let mut stream =
        TcpStream::connect(format!("127.0.0.1:{port}")).map_err(|_| ControlError::ServiceNotRunning)?;
    let payload = serde_json::to_string(cmd).map_err(|e| ControlError::Protocol(e.to_string()))?;
    writeln!(stream, "{payload}").map_err(|e| ControlError::Transport(e.to_string()))?;
    stream.flush().map_err(|e| ControlError::Transport(e.to_string()))?;

    let mut reader = BufReader::new(stream);
    let mut line = String::new();
    reader
        .read_line(&mut line)
        .map_err(|e| ControlError::Transport(e.to_string()))?;
    serde_json::from_str(line.trim()).map_err(|e| ControlError::Protocol(e.to_string()))
}

pub fn capture_service_likely_running() -> bool {
    let pid_path = paths::data_dir().join("dayrecord.pid");
    let Ok(raw) = std::fs::read_to_string(&pid_path) else {
        return false;
    };
    let Ok(pid) = raw.trim().parse::<u32>() else {
        return false;
    };
    super::instance::process_alive(pid)
}

/// In-process client for unit tests (no socket).
pub struct LoopbackControlClient<S> {
    service: Arc<S>,
}

impl<S> LoopbackControlClient<S> {
    pub fn new(service: Arc<S>) -> Self {
        Self { service }
    }
}

impl<S> ControlClient for LoopbackControlClient<S>
where
    S: super::server::ControlService,
{
    fn request(&self, cmd: ControlCommand) -> Result<ControlResponse, ControlError> {
        Ok(self.service.handle(cmd))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dayrecord_core::control::ControlData;
    use std::sync::Mutex;

    struct EchoService {
        flag: Mutex<bool>,
    }

    impl super::super::server::ControlService for EchoService {
        fn handle(&self, cmd: ControlCommand) -> ControlResponse {
            match cmd {
                ControlCommand::Resume => {
                    *self.flag.lock().unwrap() = true;
                    ControlResponse::ok(ControlData {
                        recording: Some(true),
                        day: None,
                        stats: None,
                        summary_markdown: None,
                        fact_count: None,
                    })
                }
                _ => ControlResponse::err("nope"),
            }
        }
    }

    #[test]
    fn ipc_client_reports_service_not_running_when_no_server() {
        let client = IpcControlClient;
        let err = client
            .request(ControlCommand::Status)
            .expect_err("should fail");
        assert!(matches!(err, ControlError::ServiceNotRunning));
    }

    #[test]
    fn loopback_resume_works() {
        let service = Arc::new(EchoService {
            flag: Mutex::new(false),
        });
        let client = LoopbackControlClient::new(service);
        assert!(client.request(ControlCommand::Resume).unwrap().ok);
    }
}
