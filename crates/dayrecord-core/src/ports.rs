use crate::models::{Activity, ActivityAgg, DayStats, Fact, FlowEvent, Session, Summary, TaskUnit};
use chrono::{DateTime, Utc};
use std::error::Error;

pub trait Clock: Send + Sync {
    fn now(&self) -> DateTime<Utc>;
}

pub trait KeyboardSource: Send {
    fn poll_events(&mut self) -> Vec<crate::models::KeyEvent>;
}

pub trait WindowSampler: Send + Sync {
    fn sample(&self) -> (String, String);
}

/// Screenshot-free on-screen text via UI Automation (Windows only).
pub trait ContextSampler: Send + Sync {
    fn sample_context(&self) -> Option<String>;
}

pub trait Clipboard: Send + Sync {
    fn read_text(&self) -> Result<Option<String>, Box<dyn Error + Send + Sync>>;
}

pub trait SecretStore: Send + Sync {
    fn set(&self, key: &str, value: &str) -> Result<(), Box<dyn Error + Send + Sync>>;
    fn get(&self, key: &str) -> Result<Option<String>, Box<dyn Error + Send + Sync>>;
}

pub trait LlmClient: Send + Sync {
    fn complete(&self, system: &str, user: &str) -> Result<String, Box<dyn Error + Send + Sync>>;
}

pub trait Repository: Send + Sync {
    fn insert_session(&self, session: &Session) -> Result<i64, Box<dyn Error + Send + Sync>>;
    fn list_sessions_for_day(&self, day: &str) -> Result<Vec<Session>, Box<dyn Error + Send + Sync>>;
    fn insert_activity(&self, activity: &Activity) -> Result<i64, Box<dyn Error + Send + Sync>>;
    fn list_activities_for_day(&self, day: &str) -> Result<Vec<Activity>, Box<dyn Error + Send + Sync>>;
    fn activities_for_range(&self, from: &str, to: &str) -> Result<Vec<Activity>, Box<dyn Error + Send + Sync>>;
    fn activity_agg_for_day(&self, day: &str) -> Result<Vec<ActivityAgg>, Box<dyn Error + Send + Sync>>;
    fn upsert_summary(&self, summary: &Summary) -> Result<(), Box<dyn Error + Send + Sync>>;
    fn get_summary(&self, day: &str) -> Result<Option<Summary>, Box<dyn Error + Send + Sync>>;
    fn summaries_for_range(&self, from: &str, to: &str) -> Result<Vec<Summary>, Box<dyn Error + Send + Sync>>;
    fn set_setting(&self, key: &str, value: &str) -> Result<(), Box<dyn Error + Send + Sync>>;
    fn get_setting(&self, key: &str) -> Result<Option<String>, Box<dyn Error + Send + Sync>>;
    fn clear_all_data(&self) -> Result<(), Box<dyn Error + Send + Sync>>;
    fn day_stats(&self, day: &str, pending_chars: u32) -> Result<DayStats, Box<dyn Error + Send + Sync>>;

    fn upsert_fact(
        &self,
        subject: &str,
        predicate: &str,
        object: &str,
        category: &str,
        confidence: f32,
        source_day: &str,
    ) -> Result<(), Box<dyn Error + Send + Sync>>;
    fn supersede_facts(
        &self,
        predicate: &str,
        category: &str,
        keep_object: &str,
        as_of_day: &str,
    ) -> Result<(), Box<dyn Error + Send + Sync>>;
    fn list_active_facts(&self) -> Result<Vec<Fact>, Box<dyn Error + Send + Sync>>;
    fn list_all_facts(&self) -> Result<Vec<Fact>, Box<dyn Error + Send + Sync>>;
    fn search_facts(&self, query: &str, limit: usize) -> Result<Vec<Fact>, Box<dyn Error + Send + Sync>>;
    fn delete_fact(&self, id: i64) -> Result<(), Box<dyn Error + Send + Sync>>;

    fn insert_flow_event(&self, event: &FlowEvent) -> Result<i64, Box<dyn Error + Send + Sync>>;
    fn list_flow_events_for_day(&self, day: &str) -> Result<Vec<FlowEvent>, Box<dyn Error + Send + Sync>>;

    fn replace_task_units_for_day(&self, day: &str, units: &[TaskUnit]) -> Result<(), Box<dyn Error + Send + Sync>>;
    fn list_task_units_for_day(&self, day: &str) -> Result<Vec<TaskUnit>, Box<dyn Error + Send + Sync>>;
    fn list_task_units_recent(&self, days: u32) -> Result<Vec<TaskUnit>, Box<dyn Error + Send + Sync>>;
}

#[derive(Debug, Clone)]
pub struct FixedClock {
    now: DateTime<Utc>,
}

impl FixedClock {
    pub fn new(now: DateTime<Utc>) -> Self {
        Self { now }
    }

    pub fn advance_secs(&mut self, secs: i64) {
        self.now += chrono::Duration::seconds(secs);
    }
}

impl Clock for FixedClock {
    fn now(&self) -> DateTime<Utc> {
        self.now
    }
}

/// No-op context sampler for tests and non-Windows builds.
#[derive(Debug, Default, Clone)]
pub struct NullContextSampler;

impl ContextSampler for NullContextSampler {
    fn sample_context(&self) -> Option<String> {
        None
    }
}

#[derive(Debug, Default)]
pub struct InMemoryRepository {
    pub sessions: std::sync::Mutex<Vec<Session>>,
    pub activities: std::sync::Mutex<Vec<Activity>>,
    pub summaries: std::sync::Mutex<Vec<Summary>>,
    pub settings: std::sync::Mutex<std::collections::HashMap<String, String>>,
    pub facts: std::sync::Mutex<Vec<Fact>>,
    pub flow_events: std::sync::Mutex<Vec<FlowEvent>>,
    pub task_units: std::sync::Mutex<Vec<TaskUnit>>,
}

impl Repository for InMemoryRepository {
    fn insert_session(&self, session: &Session) -> Result<i64, Box<dyn Error + Send + Sync>> {
        let mut sessions = self.sessions.lock().unwrap();
        let id = sessions.len() as i64 + 1;
        let mut s = session.clone();
        s.id = Some(id);
        sessions.push(s);
        Ok(id)
    }

    fn list_sessions_for_day(&self, day: &str) -> Result<Vec<Session>, Box<dyn Error + Send + Sync>> {
        Ok(self
            .sessions
            .lock()
            .unwrap()
            .iter()
            .filter(|s| s.day == day)
            .cloned()
            .collect())
    }

    fn insert_activity(&self, activity: &Activity) -> Result<i64, Box<dyn Error + Send + Sync>> {
        let mut activities = self.activities.lock().unwrap();
        let id = activities.len() as i64 + 1;
        let mut a = activity.clone();
        a.id = Some(id);
        activities.push(a);
        Ok(id)
    }

    fn list_activities_for_day(&self, day: &str) -> Result<Vec<Activity>, Box<dyn Error + Send + Sync>> {
        Ok(self
            .activities
            .lock()
            .unwrap()
            .iter()
            .filter(|a| a.day == day)
            .cloned()
            .collect())
    }

    fn activities_for_range(&self, from: &str, to: &str) -> Result<Vec<Activity>, Box<dyn Error + Send + Sync>> {
        Ok(self
            .activities
            .lock()
            .unwrap()
            .iter()
            .filter(|a| a.day.as_str() >= from && a.day.as_str() <= to)
            .cloned()
            .collect())
    }

    fn activity_agg_for_day(&self, day: &str) -> Result<Vec<ActivityAgg>, Box<dyn Error + Send + Sync>> {
        use std::collections::HashMap;
        let activities = self.list_activities_for_day(day)?;
        let mut map: HashMap<(String, String), (i64, Option<String>)> = HashMap::new();
        for a in activities {
            let key = (a.app_name.clone(), a.window_title.clone());
            let entry = map.entry(key).or_insert((0, None));
            entry.0 += a.seconds as i64;
            if let Some(ref snap) = a.uia_snapshot {
                let better = entry
                    .1
                    .as_ref()
                    .map(|prev| snap.chars().count() > prev.chars().count())
                    .unwrap_or(true);
                if better {
                    entry.1 = Some(snap.clone());
                }
            }
        }
        let mut out: Vec<ActivityAgg> = map
            .into_iter()
            .map(|((app_name, window_title), (seconds, uia_snapshot))| ActivityAgg {
                app_name,
                window_title,
                seconds,
                uia_snapshot,
            })
            .collect();
        out.sort_by(|a, b| b.seconds.cmp(&a.seconds));
        Ok(out)
    }

    fn upsert_summary(&self, summary: &Summary) -> Result<(), Box<dyn Error + Send + Sync>> {
        let mut summaries = self.summaries.lock().unwrap();
        if let Some(existing) = summaries.iter_mut().find(|s| s.day == summary.day) {
            *existing = summary.clone();
        } else {
            summaries.push(summary.clone());
        }
        Ok(())
    }

    fn get_summary(&self, day: &str) -> Result<Option<Summary>, Box<dyn Error + Send + Sync>> {
        Ok(self
            .summaries
            .lock()
            .unwrap()
            .iter()
            .find(|s| s.day == day)
            .cloned())
    }

    fn summaries_for_range(&self, from: &str, to: &str) -> Result<Vec<Summary>, Box<dyn Error + Send + Sync>> {
        let mut out: Vec<Summary> = self
            .summaries
            .lock()
            .unwrap()
            .iter()
            .filter(|s| s.day.as_str() >= from && s.day.as_str() <= to)
            .cloned()
            .collect();
        out.sort_by(|a, b| a.day.cmp(&b.day));
        Ok(out)
    }

    fn set_setting(&self, key: &str, value: &str) -> Result<(), Box<dyn Error + Send + Sync>> {
        self.settings
            .lock()
            .unwrap()
            .insert(key.to_string(), value.to_string());
        Ok(())
    }

    fn get_setting(&self, key: &str) -> Result<Option<String>, Box<dyn Error + Send + Sync>> {
        Ok(self.settings.lock().unwrap().get(key).cloned())
    }

    fn clear_all_data(&self) -> Result<(), Box<dyn Error + Send + Sync>> {
        self.sessions.lock().unwrap().clear();
        self.activities.lock().unwrap().clear();
        self.summaries.lock().unwrap().clear();
        self.facts.lock().unwrap().clear();
        self.flow_events.lock().unwrap().clear();
        self.task_units.lock().unwrap().clear();
        let consent = self.get_setting("consent")?;
        self.settings.lock().unwrap().clear();
        if let Some(c) = consent {
            self.set_setting("consent", &c)?;
        }
        Ok(())
    }

    fn day_stats(&self, day: &str, pending_chars: u32) -> Result<DayStats, Box<dyn Error + Send + Sync>> {
        let sessions = self.list_sessions_for_day(day)?;
        let activities = self.list_activities_for_day(day)?;
        Ok(DayStats {
            active_seconds: activities.iter().map(|a| a.seconds).sum(),
            session_count: sessions.len() as u32,
            char_count: sessions.iter().map(|s| s.content.chars().count() as u32).sum(),
            pending_chars,
        })
    }

    fn upsert_fact(
        &self,
        subject: &str,
        predicate: &str,
        object: &str,
        category: &str,
        confidence: f32,
        source_day: &str,
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        use crate::models::FactCategory;
        let cat = FactCategory::parse(category).ok_or("invalid category")?;
        let now = Utc::now();
        let mut facts = self.facts.lock().unwrap();
        if let Some(f) = facts.iter_mut().find(|f| {
            f.subject == subject && f.predicate == predicate && f.object == object
        }) {
            f.observations += 1;
            f.confidence = confidence;
            f.source_day = source_day.to_string();
            f.invalid_at = None;
            return Ok(());
        }
        let id = facts.len() as i64 + 1;
        facts.push(Fact {
            id: Some(id),
            subject: subject.to_string(),
            predicate: predicate.to_string(),
            object: object.to_string(),
            category: cat,
            confidence,
            observations: 1,
            valid_at: now,
            invalid_at: None,
            source_day: source_day.to_string(),
            created_at: now,
        });
        Ok(())
    }

    fn supersede_facts(
        &self,
        predicate: &str,
        category: &str,
        keep_object: &str,
        as_of_day: &str,
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        use crate::models::FactCategory;
        let cat = FactCategory::parse(category).ok_or("invalid category")?;
        let invalid_at = Utc::now();
        let mut facts = self.facts.lock().unwrap();
        for f in facts.iter_mut() {
            if f.invalid_at.is_none()
                && f.predicate == predicate
                && f.category == cat
                && f.object != keep_object
            {
                f.invalid_at = Some(invalid_at);
                let _ = as_of_day;
            }
        }
        Ok(())
    }

    fn list_active_facts(&self) -> Result<Vec<Fact>, Box<dyn Error + Send + Sync>> {
        Ok(self
            .facts
            .lock()
            .unwrap()
            .iter()
            .filter(|f| f.invalid_at.is_none())
            .cloned()
            .collect())
    }

    fn list_all_facts(&self) -> Result<Vec<Fact>, Box<dyn Error + Send + Sync>> {
        Ok(self.facts.lock().unwrap().clone())
    }

    fn search_facts(&self, query: &str, limit: usize) -> Result<Vec<Fact>, Box<dyn Error + Send + Sync>> {
        let q = query.to_lowercase();
        let mut hits: Vec<Fact> = self
            .list_active_facts()?
            .into_iter()
            .filter(|f| f.statement().to_lowercase().contains(&q))
            .collect();
        hits.truncate(limit);
        Ok(hits)
    }

    fn delete_fact(&self, id: i64) -> Result<(), Box<dyn Error + Send + Sync>> {
        self.facts.lock().unwrap().retain(|f| f.id != Some(id));
        Ok(())
    }

    fn insert_flow_event(&self, event: &FlowEvent) -> Result<i64, Box<dyn Error + Send + Sync>> {
        let mut events = self.flow_events.lock().unwrap();
        let id = events.len() as i64 + 1;
        let mut e = event.clone();
        e.id = Some(id);
        events.push(e);
        Ok(id)
    }

    fn list_flow_events_for_day(&self, day: &str) -> Result<Vec<FlowEvent>, Box<dyn Error + Send + Sync>> {
        Ok(self
            .flow_events
            .lock()
            .unwrap()
            .iter()
            .filter(|e| e.day == day)
            .cloned()
            .collect())
    }

    fn replace_task_units_for_day(&self, day: &str, units: &[TaskUnit]) -> Result<(), Box<dyn Error + Send + Sync>> {
        let mut all = self.task_units.lock().unwrap();
        all.retain(|u| u.day != day);
        for (i, unit) in units.iter().enumerate() {
            let mut u = unit.clone();
            u.id = Some(all.len() as i64 + i as i64 + 1);
            all.push(u);
        }
        Ok(())
    }

    fn list_task_units_for_day(&self, day: &str) -> Result<Vec<TaskUnit>, Box<dyn Error + Send + Sync>> {
        let mut units: Vec<TaskUnit> = self
            .task_units
            .lock()
            .unwrap()
            .iter()
            .filter(|u| u.day == day)
            .cloned()
            .collect();
        units.sort_by_key(|u| u.started_at);
        Ok(units)
    }

    fn list_task_units_recent(&self, days: u32) -> Result<Vec<TaskUnit>, Box<dyn Error + Send + Sync>> {
        let end = chrono::Local::now().date_naive();
        let from = (end - chrono::Duration::days(days.saturating_sub(1) as i64))
            .format("%Y-%m-%d")
            .to_string();
        let mut units: Vec<TaskUnit> = self
            .task_units
            .lock()
            .unwrap()
            .iter()
            .filter(|u| u.day.as_str() >= from.as_str())
            .cloned()
            .collect();
        units.sort_by_key(|u| u.started_at);
        Ok(units)
    }
}
