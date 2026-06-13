//! Detached daemon spawn and autostart control client.

use dayrecord_core::control::{ControlClient, ControlCommand, ControlError, ControlResponse};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};

use super::client::{capture_service_likely_running, ipc_request};
use super::server::read_control_port;

const AUTOSTART_TIMEOUT: Duration = Duration::from_secs(5);

/// Whether this command may trigger MCP autostart of `dayrecord daemon`.
pub fn command_may_autostart(cmd: &ControlCommand) -> bool {
    !matches!(cmd, ControlCommand::Status)
}

/// Spawn `dayrecord daemon` detached from the current stdio (safe for MCP parent).
pub fn spawn_detached_daemon(exe: &Path) -> Result<(), String> {
    let mut cmd = Command::new(exe);
    cmd.arg("daemon");
    cmd.stdin(Stdio::null());
    cmd.stdout(Stdio::null());
    cmd.stderr(Stdio::null());

    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        const DETACHED_PROCESS: u32 = 0x0000_0008;
        cmd.creation_flags(CREATE_NO_WINDOW | DETACHED_PROCESS);
    }

    cmd.spawn()
        .map(|_| ())
        .map_err(|e| format!("failed to spawn daemon: {e}"))
}

pub fn wait_for_capture_service(timeout: Duration) -> bool {
    let start = Instant::now();
    while start.elapsed() < timeout {
        if capture_service_likely_running() && read_control_port().is_ok() {
            return true;
        }
        thread::sleep(Duration::from_millis(100));
    }
    false
}

/// IPC client that may start `dayrecord daemon` when capture service is offline.
pub struct AutoStartControlClient {
    exe: PathBuf,
    may_autostart: Arc<dyn Fn() -> Result<(), ControlError> + Send + Sync>,
}

impl AutoStartControlClient {
    pub fn new(
        exe: PathBuf,
        may_autostart: Arc<dyn Fn() -> Result<(), ControlError> + Send + Sync>,
    ) -> Self {
        Self { exe, may_autostart }
    }
}

impl ControlClient for AutoStartControlClient {
    fn request(&self, cmd: ControlCommand) -> Result<ControlResponse, ControlError> {
        match ipc_request(&cmd) {
            Ok(resp) => return Ok(resp),
            Err(ControlError::ServiceNotRunning) if command_may_autostart(&cmd) => {}
            Err(e) => return Err(e),
        }

        (self.may_autostart)()?;
        spawn_detached_daemon(&self.exe).map_err(ControlError::Transport)?;
        if !wait_for_capture_service(AUTOSTART_TIMEOUT) {
            return Err(ControlError::ServiceNotRunning);
        }
        ipc_request(&cmd)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn status_does_not_autostart() {
        assert!(!command_may_autostart(&ControlCommand::Status));
    }

    #[test]
    fn pause_may_autostart() {
        assert!(command_may_autostart(&ControlCommand::Pause));
    }
}
