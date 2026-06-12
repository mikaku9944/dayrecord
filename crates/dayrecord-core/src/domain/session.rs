use crate::domain::ime::{append_paste, apply_key_event};
use crate::models::{KeyEvent, KeyEventKind, Session};
use crate::ports::Clock;
use chrono::{DateTime, Utc};

pub const SESSION_IDLE_SECS: i64 = 30;
pub const PASTE_MAX_CHARS: usize = 2000;

#[derive(Debug, Clone)]
pub struct SessionBuilder {
    pub app_name: String,
    pub window_title: String,
    pub buffer: String,
    pub has_paste: bool,
    pub started_at: Option<DateTime<Utc>>,
    pub last_activity_at: Option<DateTime<Utc>>,
    pub uia_text: Option<String>,
}

impl SessionBuilder {
    pub fn new(app_name: impl Into<String>, window_title: impl Into<String>) -> Self {
        Self {
            app_name: app_name.into(),
            window_title: window_title.into(),
            buffer: String::new(),
            has_paste: false,
            started_at: None,
            last_activity_at: None,
            uia_text: None,
        }
    }

    /// Keep the richest UIA snapshot seen during the pending session.
    pub fn record_uia(&mut self, snapshot: &str) {
        let trimmed = snapshot.trim();
        if trimmed.is_empty() {
            return;
        }
        let better = self
            .uia_text
            .as_deref()
            .map(|prev| trimmed.chars().count() > prev.chars().count())
            .unwrap_or(true);
        if better {
            self.uia_text = Some(trimmed.to_string());
        }
    }

    pub fn pending_char_count(&self) -> u32 {
        self.buffer.chars().count() as u32
    }

    pub fn on_key<C: Clock>(&mut self, clock: &C, event: &KeyEvent) -> Option<Session> {
        let now = event.at;
        if self.started_at.is_none() {
            self.started_at = Some(now);
        }

        if let Some(last) = self.last_activity_at {
            if (now - last).num_seconds() >= SESSION_IDLE_SECS {
                let flushed = self.flush(clock, now);
                self.started_at = Some(now);
                apply_key_event(&mut self.buffer, &event.kind);
                self.last_activity_at = Some(now);
                return flushed;
            }
        }

        if matches!(event.kind, KeyEventKind::Paste) {
            self.last_activity_at = Some(now);
            return None;
        }

        apply_key_event(&mut self.buffer, &event.kind);
        self.last_activity_at = Some(now);
        None
    }

    pub fn on_paste<C: Clock>(&mut self, clock: &C, text: &str, at: DateTime<Utc>) -> Option<Session> {
        if self.started_at.is_none() {
            self.started_at = Some(at);
        }
        append_paste(&mut self.buffer, text, PASTE_MAX_CHARS);
        self.has_paste = true;
        self.last_activity_at = Some(at);
        self.flush_if_idle(clock, at)
    }

    pub fn on_window_change<C: Clock>(
        &mut self,
        clock: &C,
        app_name: &str,
        window_title: &str,
        at: DateTime<Utc>,
    ) -> Option<Session> {
        if self.app_name == app_name && self.window_title == window_title {
            return None;
        }
        let flushed = self.flush(clock, at);
        self.app_name = app_name.to_string();
        self.window_title = window_title.to_string();
        self.started_at = None;
        self.last_activity_at = None;
        flushed
    }

    pub fn flush_if_idle<C: Clock>(&mut self, clock: &C, at: DateTime<Utc>) -> Option<Session> {
        if let Some(last) = self.last_activity_at {
            if (at - last).num_seconds() >= SESSION_IDLE_SECS {
                return self.flush(clock, at);
            }
        }
        None
    }

    pub fn flush<C: Clock>(&mut self, clock: &C, ended_at: DateTime<Utc>) -> Option<Session> {
        if self.buffer.is_empty() {
            self.reset_state();
            return None;
        }

        let started = self.started_at.unwrap_or_else(|| clock.now());
        let day = started.format("%Y-%m-%d").to_string();
        let session = Session {
            id: None,
            day,
            started_at: started,
            ended_at,
            app_name: self.app_name.clone(),
            window_title: self.window_title.clone(),
            content: std::mem::take(&mut self.buffer),
            has_paste: self.has_paste,
            uia_text: self.uia_text.take(),
        };
        self.reset_state();
        Some(session)
    }

    fn reset_state(&mut self) {
        self.buffer.clear();
        self.has_paste = false;
        self.started_at = None;
        self.last_activity_at = None;
        self.uia_text = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ports::FixedClock;
    use chrono::TimeZone;
    use rstest::rstest;

    fn clock_at(sec: i64) -> FixedClock {
        FixedClock::new(Utc.with_ymd_and_hms(2026, 6, 10, 9, 0, 0).unwrap() + chrono::Duration::seconds(sec))
    }

    #[test]
    fn merges_same_window_until_idle() {
        let mut builder = SessionBuilder::new("notepad.exe", "Untitled");
        let mut clock = clock_at(0);
        let t0 = clock.now();
        builder.on_key(
            &clock,
            &KeyEvent {
                at: t0,
                kind: KeyEventKind::Char('h'),
            },
        );
        clock.advance_secs(10);
        builder.on_key(
            &clock,
            &KeyEvent {
                at: clock.now(),
                kind: KeyEventKind::Char('i'),
            },
        );
        assert!(builder.flush(&clock, clock.now()).is_some());
        assert_eq!(builder.buffer, "");
    }

    #[rstest]
    #[case(29, false)]
    #[case(30, true)]
    fn idle_boundary(#[case] gap: i64, #[case] should_flush: bool) {
        let mut builder = SessionBuilder::new("app", "win");
        let mut clock = clock_at(0);
        builder.on_key(
            &clock,
            &KeyEvent {
                at: clock.now(),
                kind: KeyEventKind::Char('a'),
            },
        );
        clock.advance_secs(gap);
        let flushed = builder.on_key(
            &clock,
            &KeyEvent {
                at: clock.now(),
                kind: KeyEventKind::Char('b'),
            },
        );
        assert_eq!(flushed.is_some(), should_flush);
    }

    #[test]
    fn window_switch_flushes() {
        let mut builder = SessionBuilder::new("a.exe", "A");
        let mut clock = clock_at(0);
        builder.on_key(
            &clock,
            &KeyEvent {
                at: clock.now(),
                kind: KeyEventKind::Char('x'),
            },
        );
        let flushed = builder.on_window_change(&clock, "b.exe", "B", clock.now());
        assert!(flushed.is_some());
        assert_eq!(builder.app_name, "b.exe");
    }

    #[test]
    fn paste_marks_has_paste() {
        let mut builder = SessionBuilder::new("app", "win");
        let clock = clock_at(0);
        builder.on_paste(&clock, "hello", clock.now());
        let session = builder.flush(&clock, clock.now()).unwrap();
        assert!(session.has_paste);
        assert!(session.content.contains("[PASTE]"));
    }
}
