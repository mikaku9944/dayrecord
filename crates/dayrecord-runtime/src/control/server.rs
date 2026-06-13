//! Blocking TCP control server on localhost (one client at a time).

use dayrecord_core::control::{ControlCommand, ControlResponse};
use dayrecord_core::paths;
use std::io::{BufRead, BufReader, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::Arc;
use std::thread::{self, JoinHandle};

pub trait ControlService: Send + Sync + 'static {
    fn handle(&self, cmd: ControlCommand) -> ControlResponse;
}

pub fn control_port_path() -> std::path::PathBuf {
    paths::data_dir().join("control.port")
}

pub fn write_control_port(port: u16) -> Result<(), String> {
    paths::ensure_data_dir().map_err(|e| e.to_string())?;
    std::fs::write(control_port_path(), port.to_string()).map_err(|e| e.to_string())
}

pub fn read_control_port() -> Result<u16, String> {
    let raw = std::fs::read_to_string(control_port_path()).map_err(|e| e.to_string())?;
    raw.trim()
        .parse()
        .map_err(|e| format!("invalid control.port: {e}"))
}

pub fn spawn_control_server<S: ControlService>(service: Arc<S>) -> JoinHandle<()> {
    thread::spawn(move || {
        if let Err(e) = run_server(service) {
            tracing::warn!("control server stopped: {e}");
        }
    })
}

fn run_server<S: ControlService>(service: Arc<S>) -> Result<(), String> {
    let listener = TcpListener::bind("127.0.0.1:0").map_err(|e| e.to_string())?;
    let port = listener.local_addr().map_err(|e| e.to_string())?.port();
    write_control_port(port)?;
    tracing::info!("control server listening on 127.0.0.1:{port}");

    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                let _ = handle_connection(service.as_ref(), stream);
            }
            Err(e) => tracing::warn!("control accept error: {e}"),
        }
    }
    Ok(())
}

fn handle_connection<S: ControlService>(service: &S, stream: TcpStream) -> Result<(), String> {
    let mut reader = BufReader::new(stream.try_clone().map_err(|e| e.to_string())?);
    let mut line = String::new();
    reader
        .read_line(&mut line)
        .map_err(|e| e.to_string())?;
    let cmd: ControlCommand =
        serde_json::from_str(line.trim()).map_err(|e| format!("invalid command: {e}"))?;
    let resp = service.handle(cmd);
    let mut stream = stream;
    let out = serde_json::to_string(&resp).map_err(|e| e.to_string())?;
    writeln!(stream, "{out}").map_err(|e| e.to_string())?;
    stream.flush().map_err(|e| e.to_string())?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use dayrecord_core::control::{ControlClient, ControlData};
    use std::sync::Mutex;

    struct EchoService {
        recording: Mutex<bool>,
    }

    impl ControlService for EchoService {
        fn handle(&self, cmd: ControlCommand) -> ControlResponse {
            match cmd {
                ControlCommand::Pause => {
                    *self.recording.lock().unwrap() = false;
                    ControlResponse::ok(ControlData {
                        recording: Some(false),
                        day: None,
                        stats: None,
                        summary_markdown: None,
                        fact_count: None,
                    })
                }
                ControlCommand::Status => ControlResponse::ok(ControlData {
                    recording: Some(*self.recording.lock().unwrap()),
                    day: None,
                    stats: None,
                    summary_markdown: None,
                    fact_count: None,
                }),
                _ => ControlResponse::err("unsupported"),
            }
        }
    }

    #[test]
    fn loopback_client_server_roundtrip() {
        let service = Arc::new(EchoService {
            recording: Mutex::new(true),
        });
        let client = crate::control::client::LoopbackControlClient::new(service.clone());
        assert_eq!(
            client
                .request(ControlCommand::Pause)
                .unwrap()
                .data
                .and_then(|d| d.recording),
            Some(false)
        );
        let status = client.request(ControlCommand::Status).unwrap();
        assert_eq!(status.data.and_then(|d| d.recording), Some(false));
    }
}
