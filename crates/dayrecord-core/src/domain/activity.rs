use crate::models::{Activity, WindowSample};
use chrono::{DateTime, Utc};

pub const SAMPLE_INTERVAL_SECS: i64 = 5;
pub const IDLE_THRESHOLD_SECS: i64 = 60;

#[derive(Debug, Clone, Default)]
pub struct ActivityTracker {
    current_app: Option<String>,
    current_title: Option<String>,
    segment_start: Option<DateTime<Utc>>,
    last_input_at: Option<DateTime<Utc>>,
    pending_seconds: u32,
    pending_uia: Option<String>,
}

impl ActivityTracker {
    pub fn on_sample(&mut self, sample: &WindowSample, is_user_idle: bool) -> Option<Activity> {
        if is_user_idle {
            return self.flush_segment(sample.at);
        }

        self.last_input_at = Some(sample.at);

        let same = self.current_app.as_deref() == Some(sample.app_name.as_str())
            && self.current_title.as_deref() == Some(sample.window_title.as_str());

        if same {
            self.pending_seconds = self.pending_seconds.saturating_add(SAMPLE_INTERVAL_SECS as u32);
            return None;
        }

        let flushed = self.flush_segment(sample.at);
        self.current_app = Some(sample.app_name.clone());
        self.current_title = Some(sample.window_title.clone());
        self.segment_start = Some(sample.at);
        self.pending_seconds = SAMPLE_INTERVAL_SECS as u32;
        flushed
    }

    pub fn flush_segment(&mut self, ended_at: DateTime<Utc>) -> Option<Activity> {
        if self.pending_seconds == 0 {
            self.pending_uia = None;
            return None;
        }
        let started = self.segment_start.unwrap_or(ended_at);
        let day = started.format("%Y-%m-%d").to_string();
        let activity = Activity {
            id: None,
            day,
            started_at: started,
            ended_at,
            app_name: self.current_app.clone().unwrap_or_default(),
            window_title: self.current_title.clone().unwrap_or_default(),
            seconds: self.pending_seconds,
            uia_snapshot: self.pending_uia.take(),
        };
        self.pending_seconds = 0;
        self.segment_start = Some(ended_at);
        Some(activity)
    }

    pub fn record_input(&mut self, at: DateTime<Utc>) {
        self.last_input_at = Some(at);
    }

    /// Keep the richest UIA snapshot seen during the current segment.
    pub fn record_uia(&mut self, snapshot: &str) {
        let trimmed = snapshot.trim();
        if trimmed.is_empty() {
            return;
        }
        let better = self
            .pending_uia
            .as_deref()
            .map(|prev| trimmed.chars().count() > prev.chars().count())
            .unwrap_or(true);
        if better {
            self.pending_uia = Some(trimmed.to_string());
        }
    }

    pub fn is_user_idle(&self, now: DateTime<Utc>) -> bool {
        is_idle_gap(self.last_input_at, now, IDLE_THRESHOLD_SECS)
    }
}

pub fn is_idle_gap(last_input_at: Option<DateTime<Utc>>, now: DateTime<Utc>, threshold_secs: i64) -> bool {
    match last_input_at {
        None => true,
        Some(last) => (now - last).num_seconds() > threshold_secs,
    }
}

pub fn aggregate_activities(activities: &[Activity]) -> Vec<Activity> {
    use std::collections::BTreeMap;
    let mut map: BTreeMap<(String, String), u32> = BTreeMap::new();
    for a in activities {
        *map.entry((a.app_name.clone(), a.window_title.clone())).or_insert(0) += a.seconds;
    }
    map.into_iter()
        .map(|((app_name, window_title), seconds)| Activity {
            id: None,
            day: activities.first().map(|a| a.day.clone()).unwrap_or_default(),
            started_at: activities.first().map(|a| a.started_at).unwrap_or_else(Utc::now),
            ended_at: activities.first().map(|a| a.ended_at).unwrap_or_else(Utc::now),
            app_name,
            window_title,
            seconds,
            uia_snapshot: None,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;
    use rstest::rstest;

    fn ts(sec: i64) -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 6, 10, 10, 0, 0).unwrap() + chrono::Duration::seconds(sec)
    }

    #[rstest]
    #[case(59, false)]
    #[case(61, true)]
    fn idle_threshold(#[case] gap: i64, #[case] idle: bool) {
        let last = ts(0);
        assert_eq!(is_idle_gap(Some(last), ts(gap), IDLE_THRESHOLD_SECS), idle);
    }

    #[test]
    fn aggregates_same_window_samples() {
        let mut tracker = ActivityTracker::default();
        tracker.last_input_at = Some(ts(0));
        tracker.on_sample(
            &WindowSample {
                at: ts(0),
                app_name: "code.exe".into(),
                window_title: "main.rs".into(),
            },
            false,
        );
        tracker.on_sample(
            &WindowSample {
                at: ts(5),
                app_name: "code.exe".into(),
                window_title: "main.rs".into(),
            },
            false,
        );
        assert_eq!(tracker.pending_seconds, 10);
    }

    #[test]
    fn idle_skips_counting() {
        let mut tracker = ActivityTracker::default();
        tracker.last_input_at = Some(ts(0));
        tracker.pending_seconds = 5;
        tracker.segment_start = Some(ts(0));
        tracker.current_app = Some("a".into());
        tracker.current_title = Some("b".into());
        let flushed = tracker.on_sample(
            &WindowSample {
                at: ts(100),
                app_name: "a".into(),
                window_title: "b".into(),
            },
            true,
        );
        assert!(flushed.is_some());
        assert_eq!(tracker.pending_seconds, 0);
    }
}
