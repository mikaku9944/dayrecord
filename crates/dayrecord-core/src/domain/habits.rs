//! Habit profiling over the activity timeline: active hours, weekday spread,
//! top apps/projects, focus-block length and window-switch frequency.
//! Pure functions over `Activity`; the repository feeds the window of rows.

use crate::models::Activity;
use crate::time_local::{local_day_string, local_hour, local_weekday};
use std::collections::HashMap;

pub const DEFAULT_WINDOW_DAYS: i64 = 14;

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct HabitProfile {
    pub window_days: i64,
    pub active_hours: Vec<(u8, i64)>,
    pub weekday_secs: [i64; 7],
    pub top_apps: Vec<(String, i64)>,
    pub top_projects: Vec<(String, i64)>,
    pub avg_session_minutes: f64,
    pub switch_frequency: f64,
    pub peak_period: String,
}

/// Strip browser/editor suffixes from window titles to recover the real
/// document/project name for project aggregation.
pub fn normalize_title(title: &str, app_name: &str) -> String {
    let mut t = title.trim().to_string();
    if t.is_empty() {
        return app_name.to_string();
    }

    let suffixes = [
        " - Google Chrome",
        " - Microsoft Edge",
        " - Mozilla Firefox",
        " - Cursor",
        " - Visual Studio Code",
        " - Code",
        " — Cursor",
        " — Google Chrome",
    ];
    for suffix in suffixes {
        if let Some(stripped) = t.strip_suffix(suffix) {
            t = stripped.trim().to_string();
            break;
        }
    }

    if let Some(idx) = t.rfind(" - ") {
        let tail = &t[idx + 3..];
        if tail.eq_ignore_ascii_case(app_name) || tail.contains("Chrome") || tail.contains("Edge") {
            t = t[..idx].trim().to_string();
        }
    }

    if t.chars().count() > 80 {
        t = t.chars().take(80).collect();
    }

    if t.is_empty() {
        app_name.to_string()
    } else {
        t
    }
}

fn compute_peak_period(hour_buckets: &[i64; 24]) -> String {
    let total: i64 = hour_buckets.iter().sum();
    if total == 0 {
        return "暂无足够数据".to_string();
    }

    let threshold = (total as f64 * 0.08).max(300.0) as i64;
    let mut best_start = 0usize;
    let mut best_len = 0usize;
    let mut best_sum = 0i64;

    let mut i = 0;
    while i < 24 {
        if hour_buckets[i] >= threshold {
            let start = i;
            let mut sum = 0i64;
            while i < 24 && hour_buckets[i] >= threshold {
                sum += hour_buckets[i];
                i += 1;
            }
            let len = i - start;
            if sum > best_sum {
                best_sum = sum;
                best_start = start;
                best_len = len;
            }
        } else {
            i += 1;
        }
    }

    if best_len == 0 {
        let (hour, _) = hour_buckets
            .iter()
            .enumerate()
            .max_by_key(|(_, v)| *v)
            .unwrap_or((0, &0));
        return format!("{hour:02}:00 附近较活跃");
    }

    let end_hour = (best_start + best_len).min(24);
    format!(
        "{:02}:00-{:02}:00 高强度",
        best_start,
        if end_hour == 24 { 0 } else { end_hour }
    )
}

pub fn build_profile(activities: &[Activity], window_days: i64) -> HabitProfile {
    let mut hour_buckets = [0i64; 24];
    let mut weekday_secs = [0i64; 7];
    let mut app_secs: HashMap<String, i64> = HashMap::new();
    let mut project_secs: HashMap<String, i64> = HashMap::new();
    let mut total_session_secs = 0i64;
    let mut session_count = 0i64;

    for a in activities {
        let seconds = a.seconds as i64;
        let hour = local_hour(a.started_at) as usize;
        hour_buckets[hour] += seconds;
        let weekday = local_weekday(a.started_at) as usize;
        weekday_secs[weekday] += seconds;
        *app_secs.entry(a.app_name.clone()).or_insert(0) += seconds;
        let project = normalize_title(&a.window_title, &a.app_name);
        *project_secs.entry(project).or_insert(0) += seconds;
        total_session_secs += seconds;
        session_count += 1;
    }

    let mut top_apps: Vec<(String, i64)> = app_secs.into_iter().collect();
    top_apps.sort_by(|a, b| b.1.cmp(&a.1));
    top_apps.truncate(8);

    let mut top_projects: Vec<(String, i64)> = project_secs.into_iter().collect();
    top_projects.sort_by(|a, b| b.1.cmp(&a.1));
    top_projects.truncate(8);

    let active_hours: Vec<(u8, i64)> = hour_buckets
        .iter()
        .enumerate()
        .map(|(h, s)| (h as u8, *s))
        .filter(|(_, s)| *s > 0)
        .collect();

    let total_active_secs: i64 = hour_buckets.iter().sum();
    let avg_session_minutes = if session_count > 0 {
        total_session_secs as f64 / session_count as f64 / 60.0
    } else {
        0.0
    };
    let switch_frequency = if total_active_secs > 0 {
        session_count as f64 / (total_active_secs as f64 / 3600.0).max(1.0)
    } else {
        0.0
    };

    HabitProfile {
        window_days,
        active_hours,
        weekday_secs,
        top_apps,
        top_projects,
        avg_session_minutes,
        switch_frequency,
        peak_period: compute_peak_period(&hour_buckets),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::time_local::local_hour;
    use chrono::{TimeZone, Utc};

    fn act(app: &str, title: &str, hour: u32, seconds: u32) -> Activity {
        let started = Utc.with_ymd_and_hms(2026, 6, 10, hour, 0, 0).unwrap();
        Activity {
            id: None,
            day: "2026-06-10".into(),
            started_at: started,
            ended_at: started + chrono::Duration::seconds(seconds as i64),
            app_name: app.into(),
            window_title: title.into(),
            seconds,
            uia_snapshot: None,
        }
    }

    #[test]
    fn normalize_title_strips_browser_suffix() {
        let t = normalize_title("DayRecord PRD - Google Chrome", "chrome");
        assert_eq!(t, "DayRecord PRD");
    }

    #[test]
    fn peak_period_detects_block() {
        let acts: Vec<Activity> = (21..24).map(|h| act("code", "main.rs", h, 3600)).collect();
        let profile = build_profile(&acts, DEFAULT_WINDOW_DAYS);
        let expected = local_hour(acts[0].started_at);
        assert!(profile.peak_period.contains(&format!("{expected:02}:00")));
    }

    #[test]
    fn aggregates_top_apps_and_projects() {
        let acts = vec![
            act("code.exe", "main.rs - Cursor", 9, 1200),
            act("code.exe", "main.rs - Cursor", 10, 600),
            act("chrome.exe", "Docs - Google Chrome", 11, 300),
        ];
        let profile = build_profile(&acts, DEFAULT_WINDOW_DAYS);
        assert_eq!(profile.top_apps[0].0, "code.exe");
        assert_eq!(profile.top_apps[0].1, 1800);
        assert!(profile.top_projects.iter().any(|(p, _)| p == "main.rs"));
    }
}
