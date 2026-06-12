use crate::hermes::{clear_export_dir, export_all};
use crate::state::{build_orchestrator, data_dir, load_api_key, save_api_key, secret_store, AppOrchestrator};
use chrono::Utc;
use dayrecord_core::domain::habits::{build_profile, DEFAULT_WINDOW_DAYS};
use dayrecord_core::models::{DayStats, Fact, Summary};
use dayrecord_core::ports::Repository;
use std::sync::Arc;
use tauri::State;

pub struct AppState {
    pub orchestrator: Arc<AppOrchestrator>,
}

#[derive(serde::Serialize)]
pub struct AppStatus {
    pub recording: bool,
    pub consent: bool,
    pub has_api_key: bool,
    pub day: String,
    pub stats: DayStats,
}

#[tauri::command]
pub fn get_status(state: State<'_, AppState>) -> Result<AppStatus, String> {
    let store = secret_store();
    let day = Utc::now().format("%Y-%m-%d").to_string();
    let stats = state.orchestrator.day_stats(&day).map_err(|e| e.to_string())?;
    let consent = state
        .orchestrator
        .repo
        .get_setting("consent")
        .map_err(|e| e.to_string())?
        .map(|v| v == "true")
        .unwrap_or(false);
    Ok(AppStatus {
        recording: state.orchestrator.is_recording(),
        consent,
        has_api_key: load_api_key(&store).is_some(),
        day,
        stats,
    })
}

#[tauri::command]
pub fn set_recording(state: State<'_, AppState>, recording: bool) -> Result<(), String> {
    state.orchestrator.set_recording(recording);
    if !recording {
        state.orchestrator.flush_pending().map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[tauri::command]
pub fn set_consent(state: State<'_, AppState>, accepted: bool) -> Result<(), String> {
    state
        .orchestrator
        .repo
        .set_setting("consent", if accepted { "true" } else { "false" })
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn set_api_key(key: String) -> Result<(), String> {
    save_api_key(&secret_store(), &key)
}

#[tauri::command]
pub fn generate_summary(state: State<'_, AppState>, day: Option<String>) -> Result<Summary, String> {
    let day = day.unwrap_or_else(|| Utc::now().format("%Y-%m-%d").to_string());
    let summary = state
        .orchestrator
        .generate_summary(&day)
        .map_err(|e| e.to_string())?;

    let auto = state
        .orchestrator
        .repo
        .get_setting("auto_export")
        .map_err(|e| e.to_string())?
        .as_deref()
        == Some("1");
    if auto {
        if load_api_key(&secret_store()).is_some() {
            let _ = state.orchestrator.extract_facts(&day);
        }
        let _ = export_all(state.orchestrator.repo.as_ref(), &data_dir());
    }

    Ok(summary)
}

#[tauri::command]
pub fn get_summary(state: State<'_, AppState>, day: Option<String>) -> Result<Option<Summary>, String> {
    let day = day.unwrap_or_else(|| Utc::now().format("%Y-%m-%d").to_string());
    state
        .orchestrator
        .repo
        .get_summary(&day)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn clear_all_data(state: State<'_, AppState>) -> Result<(), String> {
    state.orchestrator.flush_pending().map_err(|e| e.to_string())?;
    clear_export_dir(state.orchestrator.repo.as_ref(), &data_dir()).ok();
    state.orchestrator.repo.clear_all_data().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn list_facts(state: State<'_, AppState>) -> Result<Vec<Fact>, String> {
    state
        .orchestrator
        .repo
        .list_active_facts()
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn delete_fact(state: State<'_, AppState>, id: i64) -> Result<(), String> {
    state.orchestrator.repo.delete_fact(id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn extract_facts(state: State<'_, AppState>, day: Option<String>) -> Result<usize, String> {
    let day = day.unwrap_or_else(|| Utc::now().format("%Y-%m-%d").to_string());
    state.orchestrator.extract_facts(&day).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn consolidate_facts(state: State<'_, AppState>, day: Option<String>) -> Result<Vec<Fact>, String> {
    let day = day.unwrap_or_else(|| Utc::now().format("%Y-%m-%d").to_string());
    state.orchestrator.consolidate_facts(&day).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_habit_profile(state: State<'_, AppState>) -> Result<dayrecord_core::domain::HabitProfile, String> {
    let end = Utc::now().date_naive();
    let from = (end - chrono::Duration::days(DEFAULT_WINDOW_DAYS - 1))
        .format("%Y-%m-%d")
        .to_string();
    let to = end.format("%Y-%m-%d").to_string();
    let activities = state
        .orchestrator
        .repo
        .activities_for_range(&from, &to)
        .map_err(|e| e.to_string())?;
    Ok(build_profile(&activities, DEFAULT_WINDOW_DAYS))
}

#[tauri::command]
pub fn export_hermes_memory(state: State<'_, AppState>) -> Result<String, String> {
    let path = export_all(state.orchestrator.repo.as_ref(), &data_dir()).map_err(|e| e.to_string())?;
    Ok(path.to_string_lossy().into_owned())
}

#[tauri::command]
pub fn get_hermes_export_dir(state: State<'_, AppState>) -> Result<String, String> {
    let path = crate::hermes::resolve_export_dir(state.orchestrator.repo.as_ref(), &data_dir())
        .map_err(|e| e.to_string())?;
    Ok(path.to_string_lossy().into_owned())
}

#[tauri::command]
pub fn set_hermes_export_dir(path: String, state: State<'_, AppState>) -> Result<(), String> {
    state
        .orchestrator
        .repo
        .set_setting("hermes_export_dir", path.trim())
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_auto_export(state: State<'_, AppState>) -> Result<bool, String> {
    Ok(state
        .orchestrator
        .repo
        .get_setting("auto_export")
        .map_err(|e| e.to_string())?
        .as_deref()
        == Some("1"))
}

#[tauri::command]
pub fn set_auto_export(enabled: bool, state: State<'_, AppState>) -> Result<(), String> {
    state
        .orchestrator
        .repo
        .set_setting("auto_export", if enabled { "1" } else { "0" })
        .map_err(|e| e.to_string())
}

pub fn init_state() -> Result<AppState, String> {
    let store = secret_store();
    let api_key = load_api_key(&store);
    let orchestrator = build_orchestrator(api_key)?;
    Ok(AppState { orchestrator })
}
