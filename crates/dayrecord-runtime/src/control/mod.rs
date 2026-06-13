//! Local IPC control server/client for the capture service.

mod client;
mod handler;
mod instance;
mod server;

pub use client::{IpcControlClient, LoopbackControlClient};
pub use handler::OrchestratorControlHandler;
pub use instance::{process_alive, try_acquire_instance_lock, InstanceLock};
pub use server::spawn_control_server;
