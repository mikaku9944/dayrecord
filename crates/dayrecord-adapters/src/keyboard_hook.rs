#[cfg(windows)]
pub mod win {
    use dayrecord_core::models::{KeyEvent, KeyEventKind};
    use std::sync::mpsc::Sender;
    use std::sync::Mutex;
    use std::thread;
    use windows::Win32::Foundation::{HINSTANCE, LPARAM, LRESULT, WPARAM};
    use windows::Win32::UI::WindowsAndMessaging::{
        CallNextHookEx, GetMessageW, SetWindowsHookExW, UnhookWindowsHookEx, KBDLLHOOKSTRUCT,
        WH_KEYBOARD_LL, WM_KEYDOWN, WM_SYSKEYDOWN, MSG,
    };

    use crate::keyboard::win::vk_to_kind;

    static TX: Mutex<Option<Sender<KeyEvent>>> = Mutex::new(None);

    pub fn start(tx: Sender<KeyEvent>) -> Result<(), String> {
        let mut guard = TX.lock().map_err(|e| e.to_string())?;
        if guard.is_some() {
            return Ok(());
        }
        *guard = Some(tx);
        drop(guard);

        thread::spawn(|| unsafe {
            let hook = match SetWindowsHookExW(
                WH_KEYBOARD_LL,
                Some(keyboard_proc),
                HINSTANCE::default(),
                0,
            ) {
                Ok(h) => h,
                Err(e) => {
                    eprintln!("SetWindowsHookExW failed: {e}");
                    return;
                }
            };

            let mut msg = MSG::default();
            while GetMessageW(&mut msg, None, 0, 0).as_bool() {}

            let _ = UnhookWindowsHookEx(hook);
        });
        Ok(())
    }

    unsafe extern "system" fn keyboard_proc(
        code: i32,
        wparam: WPARAM,
        lparam: LPARAM,
    ) -> LRESULT {
        if code >= 0 && (wparam.0 as u32 == WM_KEYDOWN || wparam.0 as u32 == WM_SYSKEYDOWN) {
            let info = *(lparam.0 as *const KBDLLHOOKSTRUCT);
            let vk = info.vkCode as u16;
            if let Some(kind) = vk_to_kind(vk, vk == 0xE5) {
                if !matches!(kind, KeyEventKind::ImeComposition) {
                    if let Ok(guard) = TX.lock() {
                        if let Some(tx) = guard.as_ref() {
                            let _ = tx.send(KeyEvent {
                                at: chrono::Utc::now(),
                                kind,
                            });
                        }
                    }
                }
            }
        }
        CallNextHookEx(None, code, wparam, lparam)
    }
}
