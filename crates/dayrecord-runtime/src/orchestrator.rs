use chrono::Utc;
use dayrecord_core::consolidation::parse_candidate_facts;
use dayrecord_core::domain::session::SessionBuilder;
use dayrecord_core::domain::{ActivityTracker, SAMPLE_INTERVAL_SECS};
use dayrecord_core::models::{Fact, KeyEventKind, Summary, WindowSample};
use dayrecord_core::ports::{Clock, Clipboard, ContextSampler, LlmClient, Repository, WindowSampler};
use dayrecord_core::prompt::{
    build_extraction_user_prompt, build_summary_user_prompt, EXTRACTION_SYSTEM, SUMMARY_SYSTEM,
};
use dayrecord_core::summary::normalize_summary_markdown;
use std::error::Error;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

pub struct Orchestrator<C, R, L, W, Cl, Ctx>
where
    C: Clock + 'static,
    R: Repository + 'static,
    L: LlmClient + 'static,
    W: WindowSampler + 'static,
    Cl: Clipboard + 'static,
    Ctx: ContextSampler + 'static,
{
    pub clock: Arc<C>,
    pub repo: Arc<R>,
    pub llm: Arc<L>,
    pub window_sampler: Arc<W>,
    pub clipboard: Arc<Cl>,
    pub context_sampler: Arc<Ctx>,
    recording: AtomicBool,
    session_builder: Mutex<SessionBuilder>,
    activity_tracker: Mutex<ActivityTracker>,
    last_sample_at: Mutex<Option<chrono::DateTime<Utc>>>,
}

impl<C, R, L, W, Cl, Ctx> Orchestrator<C, R, L, W, Cl, Ctx>
where
    C: Clock + 'static,
    R: Repository + 'static,
    L: LlmClient + 'static,
    W: WindowSampler + 'static,
    Cl: Clipboard + 'static,
    Ctx: ContextSampler + 'static,
{
    pub fn new(
        clock: Arc<C>,
        repo: Arc<R>,
        llm: Arc<L>,
        window_sampler: Arc<W>,
        clipboard: Arc<Cl>,
        context_sampler: Arc<Ctx>,
    ) -> Self {
        let (app, title) = window_sampler.sample();
        Self {
            clock,
            repo,
            llm,
            window_sampler,
            clipboard,
            context_sampler,
            recording: AtomicBool::new(true),
            session_builder: Mutex::new(SessionBuilder::new(app, title)),
            activity_tracker: Mutex::new(ActivityTracker::default()),
            last_sample_at: Mutex::new(None),
        }
    }

    pub fn set_recording(&self, on: bool) {
        self.recording.store(on, Ordering::SeqCst);
    }

    pub fn is_recording(&self) -> bool {
        self.recording.load(Ordering::SeqCst)
    }

    pub fn pending_chars(&self) -> u32 {
        self.session_builder.lock().unwrap().pending_char_count()
    }

    fn apply_uia_snapshot(&self, snapshot: &str) {
        self.activity_tracker.lock().unwrap().record_uia(snapshot);
        self.session_builder.lock().unwrap().record_uia(snapshot);
    }

    pub fn tick_window_sample(&self) -> Result<(), Box<dyn Error + Send + Sync>> {
        if !self.is_recording() {
            return Ok(());
        }
        let now = self.clock.now();
        let mut last = self.last_sample_at.lock().unwrap();
        if let Some(prev) = *last {
            if (now - prev).num_seconds() < SAMPLE_INTERVAL_SECS {
                return Ok(());
            }
        }
        *last = Some(now);

        if let Some(uia) = self.context_sampler.sample_context() {
            self.apply_uia_snapshot(&uia);
        }

        let (app, title) = self.window_sampler.sample();
        let mut builder = self.session_builder.lock().unwrap();
        if let Some(session) = builder.on_window_change(&*self.clock, &app, &title, now) {
            self.repo.insert_session(&session)?;
        }

        let idle = self.activity_tracker.lock().unwrap().is_user_idle(now);
        let sample = WindowSample {
            at: now,
            app_name: app,
            window_title: title,
        };
        if let Some(activity) = self.activity_tracker.lock().unwrap().on_sample(&sample, idle) {
            self.repo.insert_activity(&activity)?;
        }
        Ok(())
    }

    pub fn handle_key_event(&self, event: dayrecord_core::models::KeyEvent) -> Result<(), Box<dyn Error + Send + Sync>> {
        if !self.is_recording() {
            return Ok(());
        }
        let now = event.at;
        self.activity_tracker.lock().unwrap().record_input(now);

        if matches!(event.kind, KeyEventKind::Paste) {
            if let Some(text) = self.clipboard.read_text()? {
                let mut builder = self.session_builder.lock().unwrap();
                if let Some(session) = builder.on_paste(&*self.clock, &text, now) {
                    self.repo.insert_session(&session)?;
                }
            }
            return Ok(());
        }

        let mut builder = self.session_builder.lock().unwrap();
        if let Some(session) = builder.on_key(&*self.clock, &event) {
            self.repo.insert_session(&session)?;
        }
        Ok(())
    }

    pub fn flush_pending(&self) -> Result<(), Box<dyn Error + Send + Sync>> {
        let now = self.clock.now();
        let mut builder = self.session_builder.lock().unwrap();
        if let Some(session) = builder.flush(&*self.clock, now) {
            self.repo.insert_session(&session)?;
        }
        if let Some(activity) = self.activity_tracker.lock().unwrap().flush_segment(now) {
            self.repo.insert_activity(&activity)?;
        }
        Ok(())
    }

    /// Returns `true` if local-only mode is enabled, meaning all network
    /// activity (LLM calls for summaries / fact extraction) is blocked.
    pub fn is_local_only(&self) -> bool {
        self.repo
            .get_setting("local_only")
            .ok()
            .flatten()
            .map(|v| v == "true" || v == "1")
            .unwrap_or(false)
    }

    pub fn generate_summary(&self, day: &str) -> Result<Summary, Box<dyn Error + Send + Sync>> {
        // ── Guard: local-only mode blocks all network activity ──────────
        if self.is_local_only() {
            return Err(
                "local_only mode is enabled — network calls are disabled. \
                 Run `dayrecord local-only --disable` to allow LLM requests."
                    .into(),
            );
        }
        self.flush_pending()?;
        let activities = self.repo.activity_agg_for_day(day)?;
        let sessions = self.repo.list_sessions_for_day(day)?;
        let facts = self.repo.search_facts(day, 10)?;
        let user = build_summary_user_prompt(day, &activities, &sessions, &facts);
        let raw = self.llm.complete(SUMMARY_SYSTEM, &user)?;
        let content = normalize_summary_markdown(&raw).map_err(|e| e.to_string())?;
        let summary = Summary {
            day: day.to_string(),
            content,
            created_at: self.clock.now(),
        };
        self.repo.upsert_summary(&summary)?;
        Ok(summary)
    }

    pub fn extract_facts(&self, day: &str) -> Result<usize, Box<dyn Error + Send + Sync>> {
        // ── Guard: local-only mode blocks all network activity ──────────
        if self.is_local_only() {
            return Err(
                "local_only mode is enabled — network calls are disabled. \
                 Run `dayrecord local-only --disable` to allow LLM requests."
                    .into(),
            );
        }
        self.flush_pending()?;
        let activities = self.repo.activity_agg_for_day(day)?;
        let sessions = self.repo.list_sessions_for_day(day)?;
        if activities.is_empty() && sessions.is_empty() {
            return Err("当日无活动数据".into());
        }
        let user = build_extraction_user_prompt(day, &activities, &sessions);
        let raw = self.llm.complete(EXTRACTION_SYSTEM, &user)?;
        let candidates = parse_candidate_facts(&raw).map_err(|e| e.to_string())?;
        if candidates.is_empty() {
            return Err("未抽取到有效事实".into());
        }
        let mut count = 0usize;
        for c in &candidates {
            self.repo.upsert_fact(
                &c.subject,
                &c.predicate,
                &c.object,
                c.category.as_str(),
                c.confidence,
                day,
            )?;
            if c.category.is_singleton() {
                self.repo.supersede_facts(
                    &c.predicate,
                    c.category.as_str(),
                    &c.object,
                    day,
                )?;
            }
            count += 1;
        }
        Ok(count)
    }

    /// Backward-compatible alias for the UI command name.
    pub fn consolidate_facts(&self, day: &str) -> Result<Vec<Fact>, Box<dyn Error + Send + Sync>> {
        self.extract_facts(day)?;
        Ok(self.repo.list_active_facts()?)
    }

    pub fn day_stats(&self, day: &str) -> Result<dayrecord_core::models::DayStats, Box<dyn Error + Send + Sync>> {
        Ok(self.repo.day_stats(day, self.pending_chars())?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dayrecord_core::models::{KeyEvent, KeyEventKind, Session};
    use dayrecord_core::ports::{FixedClock, InMemoryRepository, NullContextSampler};
    use chrono::TimeZone;

    struct MockLlm;
    impl LlmClient for MockLlm {
        fn complete(&self, _: &str, _: &str) -> Result<String, Box<dyn Error + Send + Sync>> {
            Ok("## 今日概览（含大致时间分配）\nok\n## 主要工作内容（按应用/项目/场景分组，结合时长说明投入）\nok\n## 重要粘贴片段摘要\nok\n## 明日待办（能推断则列出，否则写「暂无」）\n暂无".into())
        }
    }

    struct ExtractLlm;
    impl LlmClient for ExtractLlm {
        fn complete(&self, _: &str, _: &str) -> Result<String, Box<dyn Error + Send + Sync>> {
            Ok(r#"[{"subject":"用户","predicate":"正在做项目","object":"DayRecord","category":"project","confidence":0.85}]"#.into())
        }
    }

    struct MockWindow;
    impl WindowSampler for MockWindow {
        fn sample(&self) -> (String, String) {
            ("notepad.exe".into(), "t".into())
        }
    }

    struct MockClip;
    impl Clipboard for MockClip {
        fn read_text(&self) -> Result<Option<String>, Box<dyn Error + Send + Sync>> {
            Ok(None)
        }
    }

    type TestOrch = Orchestrator<FixedClock, InMemoryRepository, MockLlm, MockWindow, MockClip, NullContextSampler>;

    fn make_orch() -> (Arc<FixedClock>, Arc<InMemoryRepository>, Arc<TestOrch>) {
        let clock = Arc::new(FixedClock::new(Utc.with_ymd_and_hms(2026, 6, 10, 9, 0, 0).unwrap()));
        let repo = Arc::new(InMemoryRepository::default());
        let orch = Arc::new(Orchestrator::new(
            clock.clone(),
            repo.clone(),
            Arc::new(MockLlm),
            Arc::new(MockWindow),
            Arc::new(MockClip),
            Arc::new(NullContextSampler),
        ));
        (clock, repo, orch)
    }

    #[test]
    fn recording_switch_blocks_events() {
        let (_, repo, orch) = make_orch();
        orch.set_recording(false);
        orch.handle_key_event(KeyEvent {
            at: Utc.with_ymd_and_hms(2026, 6, 10, 9, 0, 0).unwrap(),
            kind: KeyEventKind::Char('x'),
        })
        .unwrap();
        assert_eq!(repo.list_sessions_for_day("2026-06-10").unwrap().len(), 0);
    }

    #[test]
    fn extracts_facts() {
        let (clock, repo, _) = make_orch();
        repo.insert_session(&Session {
            id: None,
            day: "2026-06-10".into(),
            started_at: clock.now(),
            ended_at: clock.now(),
            app_name: "app".into(),
            window_title: "w".into(),
            content: "work".into(),
            has_paste: false,
            uia_text: None,
        })
        .unwrap();
        let orch = Orchestrator::new(
            clock,
            repo.clone(),
            Arc::new(ExtractLlm),
            Arc::new(MockWindow),
            Arc::new(MockClip),
            Arc::new(NullContextSampler),
        );
        let count = orch.extract_facts("2026-06-10").unwrap();
        assert!(count >= 1);
        assert!(!repo.list_active_facts().unwrap().is_empty());
    }

    #[test]
    fn generates_summary() {
        let (clock, repo, orch) = make_orch();
        repo.insert_session(&Session {
            id: None,
            day: "2026-06-10".into(),
            started_at: clock.now(),
            ended_at: clock.now(),
            app_name: "app".into(),
            window_title: "w".into(),
            content: "work".into(),
            has_paste: false,
            uia_text: None,
        })
        .unwrap();
        let summary = orch.generate_summary("2026-06-10").unwrap();
        assert!(summary.content.contains("今日概览"));
    }

    #[test]
    fn local_only_blocks_generate_summary() {
        let (clock, repo, _) = make_orch();
        repo.insert_session(&Session {
            id: None,
            day: "2026-06-10".into(),
            started_at: clock.now(),
            ended_at: clock.now(),
            app_name: "app".into(),
            window_title: "w".into(),
            content: "work".into(),
            has_paste: false,
            uia_text: None,
        })
        .unwrap();
        repo.set_setting("local_only", "true").unwrap();
        let orch = Orchestrator::new(
            clock,
            repo,
            Arc::new(MockLlm),
            Arc::new(MockWindow),
            Arc::new(MockClip),
            Arc::new(NullContextSampler),
        );
        assert!(orch.is_local_only());
        let err = orch.generate_summary("2026-06-10").unwrap_err();
        assert!(
            err.to_string().contains("local_only"),
            "unexpected error: {err}"
        );
    }

    #[test]
    fn local_only_blocks_extract_facts() {
        let (clock, repo, _) = make_orch();
        repo.insert_session(&Session {
            id: None,
            day: "2026-06-10".into(),
            started_at: clock.now(),
            ended_at: clock.now(),
            app_name: "app".into(),
            window_title: "w".into(),
            content: "work".into(),
            has_paste: false,
            uia_text: None,
        })
        .unwrap();
        repo.set_setting("local_only", "true").unwrap();
        let orch = Orchestrator::new(
            clock,
            repo,
            Arc::new(ExtractLlm),
            Arc::new(MockWindow),
            Arc::new(MockClip),
            Arc::new(NullContextSampler),
        );
        let err = orch.extract_facts("2026-06-10").unwrap_err();
        assert!(
            err.to_string().contains("local_only"),
            "unexpected error: {err}"
        );
    }

    #[test]
    fn local_only_false_allows_llm() {
        let (clock, repo, _) = make_orch();
        repo.insert_session(&Session {
            id: None,
            day: "2026-06-10".into(),
            started_at: clock.now(),
            ended_at: clock.now(),
            app_name: "app".into(),
            window_title: "w".into(),
            content: "work".into(),
            has_paste: false,
            uia_text: None,
        })
        .unwrap();
        repo.set_setting("local_only", "false").unwrap();
        let orch = Orchestrator::new(
            clock,
            repo,
            Arc::new(MockLlm),
            Arc::new(MockWindow),
            Arc::new(MockClip),
            Arc::new(NullContextSampler),
        );
        assert!(!orch.is_local_only());
        // Should succeed (MockLlm returns valid summary)
        assert!(orch.generate_summary("2026-06-10").is_ok());
    }
}
