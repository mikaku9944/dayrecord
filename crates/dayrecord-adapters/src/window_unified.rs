//! Cross-platform active window sampling via active-win-pos-rs.

use dayrecord_core::ports::WindowSampler;

pub struct ActiveWindowSampler;

impl Default for ActiveWindowSampler {
    fn default() -> Self {
        Self
    }
}

impl WindowSampler for ActiveWindowSampler {
    fn sample(&self) -> (String, String) {
        match active_win_pos_rs::get_active_window() {
            Ok(win) => {
                let app = if win.app_name.is_empty() {
                    win.process_path
                        .file_name()
                        .and_then(|s| s.to_str())
                        .unwrap_or("unknown")
                        .to_string()
                } else {
                    win.app_name
                };
                let title = if win.title.is_empty() {
                    "unknown".into()
                } else {
                    win.title
                };
                (app, title)
            }
            Err(_) => ("unknown".into(), "unknown".into()),
        }
    }
}
