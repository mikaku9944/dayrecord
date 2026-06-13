//! Local IPC control server/client for the capture service.

mod autostart;
mod client;
mod handler;
mod instance;
mod server;

pub use autostart::{
    command_may_autostart, spawn_detached_daemon, wait_for_capture_service, AutoStartControlClient,
};
pub use client::{capture_service_likely_running, ipc_request, IpcControlClient, LoopbackControlClient};
pub use handler::OrchestratorControlHandler;
pub use instance::{process_alive, try_acquire_instance_lock, InstanceLock};
pub use server::spawn_control_server;
