#[cfg(windows)]
use dayrecord_core::ports::WindowSampler;
#[cfg(windows)]
use windows::Win32::Foundation::HWND;
#[cfg(windows)]
use windows::Win32::UI::WindowsAndMessaging::{GetForegroundWindow, GetWindowTextW, GetWindowThreadProcessId};
#[cfg(windows)]
use windows::Win32::System::Threading::{OpenProcess, PROCESS_QUERY_LIMITED_INFORMATION};
#[cfg(windows)]
use windows::Win32::Foundation::CloseHandle;

#[cfg(windows)]
pub struct WinWindowSampler;

#[cfg(windows)]
impl Default for WinWindowSampler {
    fn default() -> Self {
        Self
    }
}

#[cfg(windows)]
impl WindowSampler for WinWindowSampler {
    fn sample(&self) -> (String, String) {
        unsafe {
            let hwnd: HWND = GetForegroundWindow();
            if hwnd.0.is_null() {
                return ("unknown".into(), "unknown".into());
            }

            let mut title_buf = [0u16; 512];
            let len = GetWindowTextW(hwnd, &mut title_buf);
            let title = String::from_utf16_lossy(&title_buf[..len as usize]);

            let mut pid = 0u32;
            GetWindowThreadProcessId(hwnd, Some(&mut pid));
            let app_name = process_name(pid).unwrap_or_else(|| "unknown".into());
            (app_name, title)
        }
    }
}

#[cfg(windows)]
fn process_name(pid: u32) -> Option<String> {
    unsafe {
        let handle = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid).ok()?;
        let mut buf = [0u16; 260];
        let mut size = buf.len() as u32;
        windows::Win32::System::Threading::QueryFullProcessImageNameW(
            handle,
            windows::Win32::System::Threading::PROCESS_NAME_FORMAT(0),
            windows::core::PWSTR(buf.as_mut_ptr()),
            &mut size,
        )
        .ok()?;
        let _ = CloseHandle(handle);
        let path = String::from_utf16_lossy(&buf[..size as usize]);
        Some(
            std::path::Path::new(&path)
                .file_name()
                .and_then(|s| s.to_str())
                .unwrap_or("unknown")
                .to_string(),
        )
    }
}

#[cfg(not(windows))]
pub struct StubWindowSampler;

#[cfg(not(windows))]
impl Default for StubWindowSampler {
    fn default() -> Self {
        Self
    }
}

#[cfg(not(windows))]
impl dayrecord_core::ports::WindowSampler for StubWindowSampler {
    fn sample(&self) -> (String, String) {
        ("stub".into(), "stub".into())
    }
}
