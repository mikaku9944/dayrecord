use chrono::{DateTime, Utc};
use dayrecord_core::models::{Activity, ActivityAgg, DayStats, Fact, FactCategory, Session, Summary};
use dayrecord_core::ports::Repository;
use rusqlite::{params, Connection};
use std::error::Error;
use std::path::Path;
use std::sync::Mutex;

const SCHEMA_VERSION: i32 = 2;

pub struct SqliteRepository {
    conn: Mutex<Connection>,
}

impl SqliteRepository {
    pub fn open(path: &Path) -> Result<Self, rusqlite::Error> {
        let conn = Connection::open(path)?;
        conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA foreign_keys=ON;")?;
        let repo = Self {
            conn: Mutex::new(conn),
        };
        repo.migrate()?;
        Ok(repo)
    }

    fn migrate(&self) -> Result<(), rusqlite::Error> {
        let conn = self.conn.lock().unwrap();
        let version: i32 = conn
            .pragma_query_value(None, "user_version", |r| r.get(0))
            .unwrap_or(0);
        if version != SCHEMA_VERSION {
            conn.execute_batch(
                "DROP TABLE IF EXISTS facts_fts;
                 DROP TABLE IF EXISTS facts;
                 DROP TABLE IF EXISTS sessions;
                 DROP TABLE IF EXISTS activities;
                 DROP TABLE IF EXISTS summaries;
                 DROP TABLE IF EXISTS settings;",
            )?;
            conn.execute_batch(
                r#"
                CREATE TABLE sessions (
                    id INTEGER PRIMARY KEY AUTOINCREMENT,
                    day TEXT NOT NULL,
                    started_at TEXT NOT NULL,
                    ended_at TEXT NOT NULL,
                    app_name TEXT NOT NULL,
                    window_title TEXT NOT NULL,
                    content TEXT NOT NULL,
                    has_paste INTEGER NOT NULL DEFAULT 0,
                    uia_text TEXT
                );
                CREATE INDEX IF NOT EXISTS idx_sessions_day ON sessions(day);
                CREATE TABLE activities (
                    id INTEGER PRIMARY KEY AUTOINCREMENT,
                    day TEXT NOT NULL,
                    started_at TEXT NOT NULL,
                    ended_at TEXT NOT NULL,
                    app_name TEXT NOT NULL,
                    window_title TEXT NOT NULL,
                    seconds INTEGER NOT NULL,
                    uia_snapshot TEXT
                );
                CREATE INDEX IF NOT EXISTS idx_activities_day ON activities(day);
                CREATE TABLE summaries (
                    day TEXT PRIMARY KEY,
                    content TEXT NOT NULL,
                    created_at TEXT NOT NULL
                );
                CREATE TABLE settings (
                    key TEXT PRIMARY KEY,
                    value TEXT NOT NULL
                );
                CREATE TABLE facts (
                    id INTEGER PRIMARY KEY AUTOINCREMENT,
                    subject TEXT NOT NULL,
                    predicate TEXT NOT NULL,
                    object TEXT NOT NULL,
                    category TEXT NOT NULL,
                    confidence REAL NOT NULL,
                    observations INTEGER NOT NULL DEFAULT 1,
                    valid_at TEXT NOT NULL,
                    invalid_at TEXT,
                    source_day TEXT NOT NULL,
                    created_at TEXT NOT NULL,
                    UNIQUE(subject, predicate, object)
                );
                CREATE INDEX IF NOT EXISTS idx_facts_active ON facts(invalid_at);
                CREATE INDEX IF NOT EXISTS idx_facts_category ON facts(category);
                CREATE VIRTUAL TABLE facts_fts USING fts5(subject, object);
                "#,
            )?;
            conn.pragma_update(None, "user_version", SCHEMA_VERSION)?;
        }
        Ok(())
    }

    fn parse_dt(s: &str) -> DateTime<Utc> {
        DateTime::parse_from_rfc3339(s)
            .map(|d| d.with_timezone(&Utc))
            .unwrap_or_else(|_| Utc::now())
    }

    fn format_dt(dt: DateTime<Utc>) -> String {
        dt.to_rfc3339()
    }

    fn row_to_fact(row: &rusqlite::Row<'_>) -> rusqlite::Result<Fact> {
        Ok(Fact {
            id: Some(row.get(0)?),
            subject: row.get(1)?,
            predicate: row.get(2)?,
            object: row.get(3)?,
            category: FactCategory::parse(&row.get::<_, String>(4)?)
                .unwrap_or(FactCategory::Preference),
            confidence: row.get(5)?,
            observations: row.get(6)?,
            valid_at: Self::parse_dt(&row.get::<_, String>(7)?),
            invalid_at: row
                .get::<_, Option<String>>(8)?
                .map(|s| Self::parse_dt(&s)),
            source_day: row.get(9)?,
            created_at: Self::parse_dt(&row.get::<_, String>(10)?),
        })
    }
}

impl Repository for SqliteRepository {
    fn insert_session(&self, session: &Session) -> Result<i64, Box<dyn Error + Send + Sync>> {
        let has_content = !session.content.trim().is_empty();
        let has_uia = session.uia_text.as_ref().is_some_and(|t| !t.trim().is_empty());
        if !has_content && !has_uia {
            return Ok(0);
        }
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO sessions (day, started_at, ended_at, app_name, window_title, content, has_paste, uia_text)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                session.day,
                Self::format_dt(session.started_at),
                Self::format_dt(session.ended_at),
                session.app_name,
                session.window_title,
                session.content,
                session.has_paste as i32,
                session.uia_text,
            ],
        )?;
        Ok(conn.last_insert_rowid())
    }

    fn list_sessions_for_day(&self, day: &str) -> Result<Vec<Session>, Box<dyn Error + Send + Sync>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, day, started_at, ended_at, app_name, window_title, content, has_paste, uia_text
             FROM sessions WHERE day = ?1 ORDER BY started_at",
        )?;
        let rows = stmt.query_map(params![day], |row| {
            Ok(Session {
                id: Some(row.get(0)?),
                day: row.get(1)?,
                started_at: Self::parse_dt(&row.get::<_, String>(2)?),
                ended_at: Self::parse_dt(&row.get::<_, String>(3)?),
                app_name: row.get(4)?,
                window_title: row.get(5)?,
                content: row.get(6)?,
                has_paste: row.get::<_, i32>(7)? != 0,
                uia_text: row.get(8)?,
            })
        })?;
        Ok(rows.filter_map(Result::ok).collect())
    }

    fn insert_activity(&self, activity: &Activity) -> Result<i64, Box<dyn Error + Send + Sync>> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO activities (day, started_at, ended_at, app_name, window_title, seconds, uia_snapshot)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                activity.day,
                Self::format_dt(activity.started_at),
                Self::format_dt(activity.ended_at),
                activity.app_name,
                activity.window_title,
                activity.seconds,
                activity.uia_snapshot,
            ],
        )?;
        Ok(conn.last_insert_rowid())
    }

    fn list_activities_for_day(&self, day: &str) -> Result<Vec<Activity>, Box<dyn Error + Send + Sync>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, day, started_at, ended_at, app_name, window_title, seconds, uia_snapshot
             FROM activities WHERE day = ?1 ORDER BY seconds DESC",
        )?;
        let rows = stmt.query_map(params![day], |row| {
            Ok(Activity {
                id: Some(row.get(0)?),
                day: row.get(1)?,
                started_at: Self::parse_dt(&row.get::<_, String>(2)?),
                ended_at: Self::parse_dt(&row.get::<_, String>(3)?),
                app_name: row.get(4)?,
                window_title: row.get(5)?,
                seconds: row.get::<_, i32>(6)? as u32,
                uia_snapshot: row.get(7)?,
            })
        })?;
        Ok(rows.filter_map(Result::ok).collect())
    }

    fn activities_for_range(&self, from: &str, to: &str) -> Result<Vec<Activity>, Box<dyn Error + Send + Sync>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, day, started_at, ended_at, app_name, window_title, seconds, uia_snapshot
             FROM activities WHERE day >= ?1 AND day <= ?2 ORDER BY started_at ASC",
        )?;
        let rows = stmt.query_map(params![from, to], |row| {
            Ok(Activity {
                id: Some(row.get(0)?),
                day: row.get(1)?,
                started_at: Self::parse_dt(&row.get::<_, String>(2)?),
                ended_at: Self::parse_dt(&row.get::<_, String>(3)?),
                app_name: row.get(4)?,
                window_title: row.get(5)?,
                seconds: row.get::<_, i32>(6)? as u32,
                uia_snapshot: row.get(7)?,
            })
        })?;
        Ok(rows.filter_map(Result::ok).collect())
    }

    fn activity_agg_for_day(&self, day: &str) -> Result<Vec<ActivityAgg>, Box<dyn Error + Send + Sync>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT a.app_name, a.window_title, SUM(a.seconds) as total,
                    (SELECT a2.uia_snapshot FROM activities a2
                     WHERE a2.day = ?1 AND a2.app_name = a.app_name AND a2.window_title = a.window_title
                       AND a2.uia_snapshot IS NOT NULL AND trim(a2.uia_snapshot) != ''
                     ORDER BY length(a2.uia_snapshot) DESC, a2.ended_at DESC
                     LIMIT 1) as uia_snapshot
             FROM activities a
             WHERE a.day = ?1
             GROUP BY a.app_name, a.window_title
             ORDER BY total DESC",
        )?;
        let rows = stmt.query_map(params![day], |row| {
            Ok(ActivityAgg {
                app_name: row.get(0)?,
                window_title: row.get(1)?,
                seconds: row.get(2)?,
                uia_snapshot: row.get(3)?,
            })
        })?;
        Ok(rows.filter_map(Result::ok).collect())
    }

    fn upsert_summary(&self, summary: &Summary) -> Result<(), Box<dyn Error + Send + Sync>> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO summaries (day, content, created_at) VALUES (?1, ?2, ?3)
             ON CONFLICT(day) DO UPDATE SET content = excluded.content, created_at = excluded.created_at",
            params![
                summary.day,
                summary.content,
                Self::format_dt(summary.created_at),
            ],
        )?;
        Ok(())
    }

    fn get_summary(&self, day: &str) -> Result<Option<Summary>, Box<dyn Error + Send + Sync>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare("SELECT day, content, created_at FROM summaries WHERE day = ?1")?;
        let mut rows = stmt.query(params![day])?;
        if let Some(row) = rows.next()? {
            return Ok(Some(Summary {
                day: row.get(0)?,
                content: row.get(1)?,
                created_at: Self::parse_dt(&row.get::<_, String>(2)?),
            }));
        }
        Ok(None)
    }

    fn summaries_for_range(&self, from: &str, to: &str) -> Result<Vec<Summary>, Box<dyn Error + Send + Sync>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT day, content, created_at FROM summaries
             WHERE day >= ?1 AND day <= ?2 ORDER BY day ASC",
        )?;
        let rows = stmt.query_map(params![from, to], |row| {
            Ok(Summary {
                day: row.get(0)?,
                content: row.get(1)?,
                created_at: Self::parse_dt(&row.get::<_, String>(2)?),
            })
        })?;
        Ok(rows.filter_map(Result::ok).collect())
    }

    fn set_setting(&self, key: &str, value: &str) -> Result<(), Box<dyn Error + Send + Sync>> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO settings (key, value) VALUES (?1, ?2)
             ON CONFLICT(key) DO UPDATE SET value = excluded.value",
            params![key, value],
        )?;
        Ok(())
    }

    fn get_setting(&self, key: &str) -> Result<Option<String>, Box<dyn Error + Send + Sync>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare("SELECT value FROM settings WHERE key = ?1")?;
        let mut rows = stmt.query(params![key])?;
        if let Some(row) = rows.next()? {
            return Ok(Some(row.get(0)?));
        }
        Ok(None)
    }

    fn clear_all_data(&self) -> Result<(), Box<dyn Error + Send + Sync>> {
        let consent = self.get_setting("consent")?;
        let hermes_dir = self.get_setting("hermes_export_dir")?;
        let auto_export = self.get_setting("auto_export")?;
        let conn = self.conn.lock().unwrap();
        conn.execute_batch(
            "DELETE FROM sessions;
             DELETE FROM activities;
             DELETE FROM summaries;
             DELETE FROM facts;
             DELETE FROM settings;",
        )?;
        drop(conn);
        if let Some(c) = consent {
            self.set_setting("consent", &c)?;
        }
        if let Some(d) = hermes_dir {
            self.set_setting("hermes_export_dir", &d)?;
        }
        if let Some(a) = auto_export {
            self.set_setting("auto_export", &a)?;
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
        let now = Utc::now();
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO facts(subject, predicate, object, category, confidence, observations,
                               valid_at, invalid_at, source_day, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, 1, ?6, NULL, ?6, ?7)
             ON CONFLICT(subject, predicate, object) DO UPDATE SET
               observations = observations + 1,
               confidence = excluded.confidence,
               source_day = excluded.source_day,
               invalid_at = NULL",
            params![
                subject,
                predicate,
                object,
                category,
                confidence,
                source_day,
                Self::format_dt(now),
            ],
        )?;
        let id: i64 = conn.query_row(
            "SELECT id FROM facts WHERE subject=?1 AND predicate=?2 AND object=?3",
            params![subject, predicate, object],
            |r| r.get(0),
        )?;
        conn.execute("DELETE FROM facts_fts WHERE rowid = ?1", params![id])?;
        conn.execute(
            "INSERT INTO facts_fts (rowid, subject, object) VALUES (?1, ?2, ?3)",
            params![id, subject, object],
        )?;
        Ok(())
    }

    fn supersede_facts(
        &self,
        predicate: &str,
        category: &str,
        keep_object: &str,
        as_of_day: &str,
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "UPDATE facts SET invalid_at = ?1
             WHERE predicate = ?2 AND category = ?3 AND object != ?4 AND invalid_at IS NULL",
            params![as_of_day, predicate, category, keep_object],
        )?;
        Ok(())
    }

    fn list_active_facts(&self) -> Result<Vec<Fact>, Box<dyn Error + Send + Sync>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, subject, predicate, object, category, confidence, observations,
                    valid_at, invalid_at, source_day, created_at
             FROM facts WHERE invalid_at IS NULL
             ORDER BY category, confidence DESC",
        )?;
        let rows = stmt.query_map([], Self::row_to_fact)?;
        Ok(rows.filter_map(Result::ok).collect())
    }

    fn list_all_facts(&self) -> Result<Vec<Fact>, Box<dyn Error + Send + Sync>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, subject, predicate, object, category, confidence, observations,
                    valid_at, invalid_at, source_day, created_at
             FROM facts ORDER BY valid_at ASC, id ASC",
        )?;
        let rows = stmt.query_map([], Self::row_to_fact)?;
        Ok(rows.filter_map(Result::ok).collect())
    }

    fn search_facts(&self, query: &str, limit: usize) -> Result<Vec<Fact>, Box<dyn Error + Send + Sync>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT f.id, f.subject, f.predicate, f.object, f.category, f.confidence, f.observations,
                    f.valid_at, f.invalid_at, f.source_day, f.created_at
             FROM facts_fts fts
             JOIN facts f ON f.id = fts.rowid
             WHERE facts_fts MATCH ?1 AND f.invalid_at IS NULL
             LIMIT ?2",
        )?;
        let q = query.split_whitespace().collect::<Vec<_>>().join(" OR ");
        let rows = stmt.query_map(params![q, limit as i64], Self::row_to_fact)?;
        Ok(rows.filter_map(Result::ok).collect())
    }

    fn delete_fact(&self, id: i64) -> Result<(), Box<dyn Error + Send + Sync>> {
        let conn = self.conn.lock().unwrap();
        conn.execute("DELETE FROM facts WHERE id = ?1", params![id])?;
        conn.execute("DELETE FROM facts_fts WHERE rowid = ?1", params![id])?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use tempfile::NamedTempFile;

    #[test]
    fn migrates_and_buckets_by_day() {
        let file = NamedTempFile::new().unwrap();
        let repo = SqliteRepository::open(file.path()).unwrap();
        let session = Session {
            id: None,
            day: "2026-06-10".into(),
            started_at: Utc::now(),
            ended_at: Utc::now(),
            app_name: "notepad.exe".into(),
            window_title: "t".into(),
            content: "hello".into(),
            has_paste: false,
            uia_text: None,
        };
        repo.insert_session(&session).unwrap();
        assert_eq!(repo.list_sessions_for_day("2026-06-10").unwrap().len(), 1);
    }

    #[test]
    fn clear_preserves_consent() {
        let file = NamedTempFile::new().unwrap();
        let repo = SqliteRepository::open(file.path()).unwrap();
        repo.set_setting("consent", "true").unwrap();
        repo.clear_all_data().unwrap();
        assert_eq!(repo.get_setting("consent").unwrap(), Some("true".into()));
    }

    #[test]
    fn upsert_and_supersede_facts() {
        let file = NamedTempFile::new().unwrap();
        let repo = SqliteRepository::open(file.path()).unwrap();
        repo.upsert_fact("用户", "正在做项目", "A", "project", 0.9, "2026-06-10")
            .unwrap();
        repo.upsert_fact("用户", "正在做项目", "B", "project", 0.8, "2026-06-11")
            .unwrap();
        repo.supersede_facts("正在做项目", "project", "B", "2026-06-11")
            .unwrap();
        let all = repo.list_all_facts().unwrap();
        assert_eq!(all.len(), 2);
        let active = repo.list_active_facts().unwrap();
        assert_eq!(active.len(), 1);
        assert_eq!(active[0].object, "B");
    }

    #[test]
    fn activity_agg_includes_uia() {
        let file = NamedTempFile::new().unwrap();
        let repo = SqliteRepository::open(file.path()).unwrap();
        let now = Utc::now();
        repo.insert_activity(&Activity {
            id: None,
            day: "2026-06-10".into(),
            started_at: now,
            ended_at: now,
            app_name: "code.exe".into(),
            window_title: "main.rs".into(),
            seconds: 60,
            uia_snapshot: Some("short".into()),
        })
        .unwrap();
        repo.insert_activity(&Activity {
            id: None,
            day: "2026-06-10".into(),
            started_at: now,
            ended_at: now,
            app_name: "code.exe".into(),
            window_title: "main.rs".into(),
            seconds: 120,
            uia_snapshot: Some("longer uia snapshot".into()),
        })
        .unwrap();
        let agg = repo.activity_agg_for_day("2026-06-10").unwrap();
        assert_eq!(agg[0].seconds, 180);
        assert!(agg[0].uia_snapshot.as_deref().unwrap().contains("longer"));
    }
}
