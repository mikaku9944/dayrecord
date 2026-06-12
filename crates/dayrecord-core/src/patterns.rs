//! Deterministic behavioral pattern analysis over activities and sessions.

use crate::domain::habits::normalize_title;
use crate::models::{Activity, Session};
use chrono::{Datelike, Timelike};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

pub const RHYTHM_MIN_RATE: f64 = 0.6;
pub const REPEAT_MIN_DAYS: usize = 3;
pub const TASK_GAP_SECS: i64 = 600;
pub const FLOW_PREVIEW_CHARS: usize = 80;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RhythmPattern {
    pub app_name: String,
    pub hour: u8,
    pub weekday: Option<u8>,
    pub occurrence_rate: f64,
    pub description: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RepeatPattern {
    pub steps: Vec<String>,
    pub day_count: usize,
    pub description: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TaskSegment {
    pub started_at: chrono::DateTime<chrono::Utc>,
    pub ended_at: chrono::DateTime<chrono::Utc>,
    pub app_chain: Vec<String>,
    pub total_seconds: u32,
    pub uia_summary: Option<String>,
    pub paste_snippets: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HesitationScore {
    pub score: f32,
    pub backspace_rate: f32,
    pub max_pause_secs: i64,
    pub aba_switches: u32,
}

pub fn detect_rhythms(activities: &[Activity], window_days: i64) -> Vec<RhythmPattern> {
    if activities.is_empty() || window_days <= 0 {
        return vec![];
    }

    let days: HashSet<String> = activities.iter().map(|a| a.day.clone()).collect();
    let total_days = days.len().max(1);
    let mut days_by_weekday: HashMap<u8, HashSet<String>> = HashMap::new();

    let mut daily_hour_app: HashMap<(String, u8, u8), HashSet<String>> = HashMap::new();
    let mut daily_weekday_hour_app: HashMap<(String, u8, u8), HashSet<String>> = HashMap::new();

    for a in activities {
        let weekday = a.started_at.weekday().num_days_from_monday() as u8;
        days_by_weekday
            .entry(weekday)
            .or_default()
            .insert(a.day.clone());
        if a.seconds < 60 {
            continue;
        }
        let hour = a.started_at.hour() as u8;
        let weekday = a.started_at.weekday().num_days_from_monday() as u8;
        daily_hour_app
            .entry((a.day.clone(), hour, weekday))
            .or_default()
            .insert(a.app_name.clone());
        daily_weekday_hour_app
            .entry((a.app_name.clone(), hour, weekday))
            .or_default()
            .insert(a.day.clone());
    }

    let mut patterns = Vec::new();

    let mut app_hour_days: HashMap<(String, u8), HashSet<String>> = HashMap::new();
    for ((day, hour, _), apps) in &daily_hour_app {
        for app in apps {
            app_hour_days
                .entry((app.clone(), *hour))
                .or_default()
                .insert(day.clone());
        }
    }

    for ((app, hour), seen_days) in app_hour_days {
        let rate = seen_days.len() as f64 / total_days as f64;
        if rate >= RHYTHM_MIN_RATE {
            patterns.push(RhythmPattern {
                app_name: app.clone(),
                hour,
                weekday: None,
                occurrence_rate: rate,
                description: format!(
                    "每天 {:02}:00 附近固定使用 {}",
                    hour, app
                ),
            });
        }
    }

    for ((app, hour, weekday), seen_days) in daily_weekday_hour_app {
        let weekday_total = days_by_weekday
            .get(&weekday)
            .map(|d| d.len())
            .unwrap_or(1)
            .max(1);
        let rate = seen_days.len() as f64 / weekday_total as f64;
        if rate >= RHYTHM_MIN_RATE {
            let weekday_name = weekday_label(weekday);
            patterns.push(RhythmPattern {
                app_name: app.clone(),
                hour,
                weekday: Some(weekday),
                occurrence_rate: rate,
                description: format!(
                    "每{} {:02}:00 附近固定使用 {}",
                    weekday_name, hour, app
                ),
            });
        }
    }

    patterns.sort_by(|a, b| b.occurrence_rate.partial_cmp(&a.occurrence_rate).unwrap());
    patterns.dedup_by(|a, b| {
        a.app_name == b.app_name && a.hour == b.hour && a.weekday == b.weekday
    });
    patterns
}

pub fn mine_repeats(activities: &[Activity]) -> Vec<RepeatPattern> {
    let mut by_day: HashMap<String, Vec<String>> = HashMap::new();

    let mut sorted: Vec<&Activity> = activities.iter().collect();
    sorted.sort_by_key(|a| a.started_at);

    for a in sorted {
        if a.seconds < 30 {
            continue;
        }
        let step = format!(
            "{}::{}",
            a.app_name,
            normalize_title(&a.window_title, &a.app_name)
        );
        let entry = by_day.entry(a.day.clone()).or_default();
        if entry.last() != Some(&step) {
            entry.push(step);
        }
    }

    let mut ngram_days: HashMap<Vec<String>, HashSet<String>> = HashMap::new();
    for (day, steps) in &by_day {
        for n in 2..=4usize {
            if steps.len() < n {
                continue;
            }
            for window in steps.windows(n) {
                ngram_days
                    .entry(window.to_vec())
                    .or_default()
                    .insert(day.clone());
            }
        }
    }

    let mut patterns: Vec<RepeatPattern> = ngram_days
        .into_iter()
        .filter(|(_, days)| days.len() >= REPEAT_MIN_DAYS)
        .map(|(steps, days)| {
            let labels: Vec<String> = steps
                .iter()
                .map(|s| s.split_once("::").map(|(a, t)| format!("{a} ({t})")).unwrap_or_else(|| s.clone()))
                .collect();
            RepeatPattern {
                day_count: days.len(),
                description: format!("重复工作流：{}", labels.join(" → ")),
                steps,
            }
        })
        .collect();

    patterns.sort_by(|a, b| b.day_count.cmp(&a.day_count));
    patterns
}

pub fn segment_tasks(activities: &[Activity], sessions: &[Session]) -> Vec<TaskSegment> {
    let mut sorted: Vec<&Activity> = activities.iter().collect();
    sorted.sort_by_key(|a| a.started_at);
    if sorted.is_empty() {
        return vec![];
    }

    let mut segments: Vec<Vec<&Activity>> = Vec::new();
    let mut current: Vec<&Activity> = vec![sorted[0]];

    for a in sorted.iter().skip(1) {
        let prev = current.last().unwrap();
        let gap = (*a).started_at.signed_duration_since(prev.ended_at).num_seconds();
        if gap > TASK_GAP_SECS {
            segments.push(current);
            current = vec![a];
        } else {
            current.push(a);
        }
    }
    if !current.is_empty() {
        segments.push(current);
    }

    segments
        .into_iter()
        .filter_map(|acts| {
            if acts.is_empty() {
                return None;
            }
            let started_at = acts.first()?.started_at;
            let ended_at = acts.last()?.ended_at;
            let total_seconds: u32 = acts.iter().map(|a| a.seconds).sum();
            if total_seconds < 120 {
                return None;
            }

            let mut app_chain = Vec::new();
            for a in &acts {
                if app_chain.last() != Some(&a.app_name) {
                    app_chain.push(a.app_name.clone());
                }
            }

            let uia_summary = acts
                .iter()
                .filter_map(|a| a.uia_snapshot.as_ref())
                .max_by_key(|t| t.chars().count())
                .cloned();

            let paste_snippets: Vec<String> = sessions
                .iter()
                .filter(|s| s.has_paste && s.started_at >= started_at && s.ended_at <= ended_at)
                .filter_map(|s| {
                    s.content
                        .split("[PASTE]")
                        .nth(1)
                        .map(|p| p.trim().chars().take(FLOW_PREVIEW_CHARS).collect())
                })
                .filter(|p: &String| !p.is_empty())
                .take(3)
                .collect();

            Some(TaskSegment {
                started_at,
                ended_at,
                app_chain,
                total_seconds,
                uia_summary,
                paste_snippets,
            })
        })
        .collect()
}

pub fn hesitation_metrics(segment: &TaskSegment, sessions: &[Session]) -> HesitationScore {
    let overlapping: Vec<&Session> = sessions
        .iter()
        .filter(|s| s.ended_at >= segment.started_at && s.started_at <= segment.ended_at)
        .collect();

    let mut total_chars = 0u32;
    let mut backspaces = 0u32;
    let mut max_pause = 0i64;

    let mut sorted_sessions: Vec<&Session> = overlapping;
    sorted_sessions.sort_by_key(|s| s.started_at);

    for (i, s) in sorted_sessions.iter().enumerate() {
        let chars = s.content.chars().count() as u32;
        total_chars += chars;
        backspaces += s.backspace_count;
        if i > 0 {
            let gap = s
                .started_at
                .signed_duration_since(sorted_sessions[i - 1].ended_at)
                .num_seconds();
            max_pause = max_pause.max(gap);
        }
    }

    let backspace_rate = if total_chars > 0 {
        backspaces as f32 / total_chars as f32
    } else {
        0.0
    };

    let mut aba_switches = 0u32;
    for window in segment.app_chain.windows(3) {
        if window[0] == window[2] && window[0] != window[1] {
            aba_switches += 1;
        }
    }

    let pause_component = (max_pause as f32 / 300.0).min(1.0);
    let backspace_component = (backspace_rate * 5.0).min(1.0);
    let aba_component = (aba_switches as f32 / 3.0).min(1.0);
    let score = (pause_component * 0.4 + backspace_component * 0.35 + aba_component * 0.25).min(1.0);

    HesitationScore {
        score,
        backspace_rate,
        max_pause_secs: max_pause,
        aba_switches,
    }
}

pub fn rhythm_to_fact(r: &RhythmPattern) -> (String, String, String, f32) {
    (
        "用户".into(),
        if r.weekday.is_some() {
            "每周固定时段使用".into()
        } else {
            "每天固定时段使用".into()
        },
        r.description.clone(),
        r.occurrence_rate as f32,
    )
}

pub fn repeat_to_fact(r: &RepeatPattern) -> (String, String, String, f32) {
    let confidence = (r.day_count as f32 / 7.0).min(1.0);
    (
        "用户".into(),
        "重复工作流".into(),
        r.description.clone(),
        confidence,
    )
}

pub fn format_app_chain(chain: &[String]) -> String {
    serde_json::to_string(chain).unwrap_or_else(|_| chain.join(", "))
}

fn weekday_label(weekday: u8) -> &'static str {
    match weekday {
        0 => "周一",
        1 => "周二",
        2 => "周三",
        3 => "周四",
        4 => "周五",
        5 => "周六",
        _ => "周日",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{TimeZone, Utc};

    fn act(day: &str, app: &str, title: &str, hour: u32, seconds: u32) -> Activity {
        let started = Utc.with_ymd_and_hms(2026, 6, 10, hour, 0, 0).unwrap();
        Activity {
            id: None,
            day: day.into(),
            started_at: started,
            ended_at: started + chrono::Duration::seconds(seconds as i64),
            app_name: app.into(),
            window_title: title.into(),
            seconds,
            uia_snapshot: None,
        }
    }

    fn session_at(hour: u32, content: &str, backspaces: u32) -> Session {
        let started = Utc.with_ymd_and_hms(2026, 6, 10, hour, 0, 0).unwrap();
        Session {
            id: None,
            day: "2026-06-10".into(),
            started_at: started,
            ended_at: started + chrono::Duration::minutes(5),
            app_name: "code.exe".into(),
            window_title: "main.rs".into(),
            content: content.into(),
            has_paste: content.contains("[PASTE]"),
            uia_text: None,
            backspace_count: backspaces,
        }
    }

    #[test]
    fn detects_daily_rhythm() {
        let acts: Vec<Activity> = (0..5)
            .map(|i| act(&format!("2026-06-{:02}", 10 + i), "code.exe", "main.rs", 9, 3600))
            .collect();
        let rhythms = detect_rhythms(&acts, 5);
        assert!(rhythms.iter().any(|r| r.app_name == "code.exe" && r.hour == 9));
    }

    #[test]
    fn mines_repeat_sequence() {
        let mut acts = Vec::new();
        for day in 10..=13 {
            let d = format!("2026-06-{day}");
            acts.push(act(&d, "chrome.exe", "Docs - Google Chrome", 10, 300));
            acts.push(act(&d, "code.exe", "main.rs - Cursor", 10, 600));
        }
        let repeats = mine_repeats(&acts);
        assert!(repeats.iter().any(|r| r.day_count >= 3));
    }

    #[test]
    fn segments_by_gap() {
        let acts = vec![
            act("2026-06-10", "code.exe", "a", 9, 600),
            act("2026-06-10", "code.exe", "b", 9, 30),
            act("2026-06-10", "chrome.exe", "c", 11, 600),
        ];
        let segments = segment_tasks(&acts, &[]);
        assert_eq!(segments.len(), 2);
    }

    #[test]
    fn hesitation_scores_backspace() {
        let segment = TaskSegment {
            started_at: Utc.with_ymd_and_hms(2026, 6, 10, 9, 0, 0).unwrap(),
            ended_at: Utc.with_ymd_and_hms(2026, 6, 10, 10, 0, 0).unwrap(),
            app_chain: vec!["a.exe".into(), "b.exe".into(), "a.exe".into()],
            total_seconds: 3600,
            uia_summary: None,
            paste_snippets: vec![],
        };
        let sessions = vec![session_at(9, "hello world", 10)];
        let h = hesitation_metrics(&segment, &sessions);
        assert!(h.score > 0.0);
        assert!(h.backspace_rate > 0.0);
        assert_eq!(h.aba_switches, 1);
    }
}
