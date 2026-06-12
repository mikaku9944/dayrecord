mod commands;
mod hermes;
pub mod orchestrator;
mod state;

pub use commands::{init_state, AppState};
pub use orchestrator::Orchestrator;

use commands::{
    clear_all_data, consolidate_facts, delete_fact, export_hermes_memory, extract_facts,
    generate_summary, get_auto_export, get_habit_profile, get_hermes_export_dir, get_status,
    get_summary, is_job_busy, list_facts, list_task_units, set_api_key, set_auto_export,
    set_consent, set_hermes_export_dir, set_recording, start_extract_insights,
    start_generate_summary,
};
use tauri::menu::{Menu, MenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{Emitter, Manager};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let state = init_state().expect("failed to init app state");

    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .manage(state)
        .setup(|app| {
            let open_i = MenuItem::with_id(app, "open", "打开主窗口", true, None::<&str>)?;
            let pause_i = MenuItem::with_id(app, "pause", "暂停/继续录制", true, None::<&str>)?;
            let quit_i = MenuItem::with_id(app, "quit", "退出", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&open_i, &pause_i, &quit_i])?;

            let _tray = TrayIconBuilder::new()
                .menu(&menu)
                .on_menu_event(|app, event| match event.id.as_ref() {
                    "open" => {
                        if let Some(w) = app.get_webview_window("main") {
                            let _ = w.show();
                            let _ = w.set_focus();
                        }
                    }
                    "pause" => {
                        let state = app.state::<AppState>();
                        let next = !state.orchestrator.is_recording();
                        state.orchestrator.set_recording(next);
                        let _ = app.emit("recording-changed", next);
                    }
                    "quit" => {
                        app.exit(0);
                    }
                    _ => {}
                })
                .on_tray_icon_event(|tray, event| {
                    if let TrayIconEvent::Click {
                        button: MouseButton::Left,
                        button_state: MouseButtonState::Up,
                        ..
                    } = event
                    {
                        let app = tray.app_handle();
                        if let Some(w) = app.get_webview_window("main") {
                            let _ = w.show();
                            let _ = w.set_focus();
                        }
                    }
                })
                .build(app)?;

            let orch = app.state::<AppState>().orchestrator.clone();
            std::thread::spawn(move || {
                loop {
                    let _ = orch.tick_window_sample();
                    std::thread::sleep(std::time::Duration::from_secs(1));
                }
            });

            #[cfg(windows)]
            {
                use std::sync::mpsc;
                let (tx, rx) = mpsc::channel();
                let orch_kb = app.state::<AppState>().orchestrator.clone();
                let _ = dayrecord_adapters::start_keyboard_capture(tx);
                std::thread::spawn(move || {
                    loop {
                        for event in rx.try_iter() {
                            let _ = orch_kb.handle_key_event(event);
                        }
                        std::thread::sleep(std::time::Duration::from_millis(10));
                    }
                });
            }

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_status,
            set_recording,
            set_consent,
            set_api_key,
            generate_summary,
            start_generate_summary,
            get_summary,
            clear_all_data,
            list_facts,
            list_task_units,
            delete_fact,
            consolidate_facts,
            extract_facts,
            start_extract_insights,
            is_job_busy,
            get_habit_profile,
            export_hermes_memory,
            get_hermes_export_dir,
            set_hermes_export_dir,
            get_auto_export,
            set_auto_export,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
