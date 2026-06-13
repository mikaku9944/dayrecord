//! Handle control commands against a running orchestrator.

use crate::orchestrator::Orchestrator;
use chrono::Utc;
use dayrecord_core::context::summary_markdown;
use dayrecord_core::control::{ControlCommand, ControlData, ControlResponse};
use dayrecord_core::ports::{Clipboard, Clock, ContextSampler, LlmClient, Repository, WindowSampler};
use dayrecord_core::redact::sanitize;

pub struct OrchestratorControlHandler<C, R, L, W, Cl, Ctx>
where
    C: Clock + 'static,
    R: Repository + 'static,
    L: LlmClient + 'static,
    W: WindowSampler + 'static,
    Cl: Clipboard + 'static,
    Ctx: ContextSampler + 'static,
{
    pub orchestrator: std::sync::Arc<Orchestrator<C, R, L, W, Cl, Ctx>>,
}

impl<C, R, L, W, Cl, Ctx> OrchestratorControlHandler<C, R, L, W, Cl, Ctx>
where
    C: Clock + 'static,
    R: Repository + 'static,
    L: LlmClient + 'static,
    W: WindowSampler + 'static,
    Cl: Clipboard + 'static,
    Ctx: ContextSampler + 'static,
{
    pub fn dispatch(&self, cmd: ControlCommand) -> ControlResponse {
        match cmd {
            ControlCommand::Pause => {
                self.orchestrator.set_recording(false);
                if let Err(e) = self.orchestrator.flush_pending() {
                    return ControlResponse::err(e.to_string());
                }
                let _ = self
                    .orchestrator
                    .repo
                    .set_setting("recording", "false");
                ControlResponse::ok(ControlData {
                    recording: Some(false),
                    day: None,
                    stats: None,
                    summary_markdown: None,
                    fact_count: None,
                })
            }
            ControlCommand::Resume => {
                self.orchestrator.set_recording(true);
                let _ = self
                    .orchestrator
                    .repo
                    .set_setting("recording", "true");
                ControlResponse::ok(ControlData {
                    recording: Some(true),
                    day: None,
                    stats: None,
                    summary_markdown: None,
                    fact_count: None,
                })
            }
            ControlCommand::Status => match self.status_data() {
                Ok(data) => ControlResponse::ok(data),
                Err(e) => ControlResponse::err(e),
            },
            ControlCommand::GenerateSummary { day } => {
                let day = day.unwrap_or_else(today_string);
                match self.orchestrator.generate_summary(&day) {
                    Ok(summary) => ControlResponse::ok(ControlData {
                        recording: Some(self.orchestrator.is_recording()),
                        day: Some(day),
                        stats: None,
                        summary_markdown: Some(sanitize(&summary_markdown(&summary))),
                        fact_count: None,
                    }),
                    Err(e) => ControlResponse::err(e.to_string()),
                }
            }
            ControlCommand::Consolidate { day } => {
                let day = day.unwrap_or_else(today_string);
                match self.orchestrator.consolidate_facts(&day) {
                    Ok(facts) => ControlResponse::ok(ControlData {
                        recording: Some(self.orchestrator.is_recording()),
                        day: Some(day),
                        stats: None,
                        summary_markdown: None,
                        fact_count: Some(facts.len()),
                    }),
                    Err(e) => ControlResponse::err(e.to_string()),
                }
            }
        }
    }

    fn status_data(&self) -> Result<ControlData, String> {
        let day = today_string();
        let stats = self
            .orchestrator
            .day_stats(&day)
            .map_err(|e| e.to_string())?;
        Ok(ControlData {
            recording: Some(self.orchestrator.is_recording()),
            day: Some(day),
            stats: Some(stats),
            summary_markdown: None,
            fact_count: None,
        })
    }
}

impl<C, R, L, W, Cl, Ctx> crate::control::server::ControlService
    for OrchestratorControlHandler<C, R, L, W, Cl, Ctx>
where
    C: Clock + 'static,
    R: Repository + 'static,
    L: LlmClient + 'static,
    W: WindowSampler + 'static,
    Cl: Clipboard + 'static,
    Ctx: ContextSampler + 'static,
{
    fn handle(&self, cmd: ControlCommand) -> ControlResponse {
        self.dispatch(cmd)
    }
}

fn today_string() -> String {
    Utc::now().format("%Y-%m-%d").to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;
    use dayrecord_core::models::{Activity, Session};
    use dayrecord_core::ports::{FixedClock, InMemoryRepository, NullContextSampler};
    use std::sync::Arc;

    struct MockLlm;
    impl LlmClient for MockLlm {
        fn complete(
            &self,
            _: &str,
            _: &str,
        ) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
            Ok("## 今日概览（含大致时间分配）\nok\n## 主要工作内容（按应用/项目/场景分组，结合时长说明投入）\nok\n## 重要粘贴片段摘要\nok\n## 明日待办（能推断则列出，否则写「暂无」）\n暂无".into())
        }
    }

    struct MockWindow;
    impl WindowSampler for MockWindow {
        fn sample(&self) -> (String, String) {
            ("app".into(), "title".into())
        }
    }

    struct MockClipboard;
    impl Clipboard for MockClipboard {
        fn read_text(&self) -> Result<Option<String>, Box<dyn std::error::Error + Send + Sync>> {
            Ok(None)
        }
    }

    fn handler() -> OrchestratorControlHandler<
        FixedClock,
        InMemoryRepository,
        MockLlm,
        MockWindow,
        MockClipboard,
        NullContextSampler,
    > {
        let t0 = chrono::Utc.with_ymd_and_hms(2026, 6, 13, 9, 0, 0).unwrap();
        let repo = Arc::new(InMemoryRepository::default());
        repo.insert_session(&Session {
            id: None,
            day: "2026-06-13".into(),
            started_at: t0,
            ended_at: t0,
            app_name: "app".into(),
            window_title: "w".into(),
            content: "hello".into(),
            has_paste: false,
            uia_text: None,
            backspace_count: 0,
        })
        .unwrap();
        repo.insert_activity(&Activity {
            id: None,
            day: "2026-06-13".into(),
            started_at: t0,
            ended_at: t0,
            app_name: "app".into(),
            window_title: "w".into(),
            seconds: 60,
            uia_snapshot: None,
        })
        .unwrap();
        let orch = Arc::new(Orchestrator::new(
            Arc::new(FixedClock::new(t0)),
            repo,
            Arc::new(MockLlm),
            Arc::new(MockWindow),
            Arc::new(MockClipboard),
            Arc::new(NullContextSampler),
        ));
        OrchestratorControlHandler { orchestrator: orch }
    }

    #[test]
    fn pause_resume_status_roundtrip() {
        let h = handler();
        assert!(h.dispatch(ControlCommand::Pause).ok);
        assert_eq!(
            h.dispatch(ControlCommand::Status)
                .data
                .and_then(|d| d.recording),
            Some(false)
        );
        assert!(h.dispatch(ControlCommand::Resume).ok);
        assert_eq!(
            h.dispatch(ControlCommand::Status)
                .data
                .and_then(|d| d.recording),
            Some(true)
        );
    }

    #[test]
    fn generate_summary_persists() {
        let h = handler();
        let resp = h.dispatch(ControlCommand::GenerateSummary { day: None });
        assert!(resp.ok);
        assert!(resp.data.unwrap().summary_markdown.is_some());
    }
}
