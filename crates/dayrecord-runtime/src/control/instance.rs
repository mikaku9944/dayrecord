//! Single-instance lock for the capture service.

use dayrecord_core::paths;
use std::path::PathBuf;

pub struct InstanceLock {
    path: PathBuf,
}

pub fn try_acquire_instance_lock() -> Result<InstanceLock, String> {
    let path = paths::data_dir().join("dayrecord.pid");
    if path.exists() {
        if let Ok(pid_str) = std::fs::read_to_string(&path) {
            if let Ok(pid) = pid_str.trim().parse::<u32>() {
                if process_alive(pid) {
                    return Err(format!(
                        "DayRecord capture service already running (pid {pid})"
                    ));
                }
            }
        }
        let _ = std::fs::remove_file(&path);
    }
    std::fs::write(&path, std::process::id().to_string())
        .map_err(|e| format!("write pid file: {e}"))?;
    Ok(InstanceLock { path })
}

impl Drop for InstanceLock {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.path);
    }
}

pub fn process_alive(pid: u32) -> bool {
    #[cfg(unix)]
    {
        // kill(pid, 0) checks process existence without libc on Windows builds.
        std::process::Command::new("kill")
            .args(["-0", &pid.to_string()])
            .status()
            .map(|s| s.success())
            .unwrap_or(false)
    }
    #[cfg(windows)]
    {
        extern "system" {
            fn OpenProcess(dwDesiredAccess: u32, bInheritHandle: i32, dwProcessId: u32) -> *mut std::ffi::c_void;
            fn CloseHandle(hObject: *mut std::ffi::c_void) -> i32;
        }
        const PROCESS_QUERY_LIMITED_INFORMATION: u32 = 0x1000;
        unsafe {
            let handle = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid);
            if handle.is_null() {
                return false;
            }
            CloseHandle(handle);
            true
        }
    }
    #[cfg(not(any(unix, windows)))]
    {
        let _ = pid;
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn current_process_is_alive() {
        assert!(process_alive(std::process::id()));
    }

    #[test]
    fn acquire_and_release_lock() {
        let _ = paths::ensure_data_dir();
        let lock = try_acquire_instance_lock().expect("lock");
        assert!(paths::data_dir().join("dayrecord.pid").exists());
        drop(lock);
        assert!(!paths::data_dir().join("dayrecord.pid").exists());
    }
}
