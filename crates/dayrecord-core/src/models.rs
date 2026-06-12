use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Session {
    pub id: Option<i64>,
    pub day: String,
    pub started_at: DateTime<Utc>,
    pub ended_at: DateTime<Utc>,
    pub app_name: String,
    pub window_title: String,
    pub content: String,
    pub has_paste: bool,
    /// Screenshot-free on-screen text captured via UIA at flush time (optional).
    #[serde(default)]
    pub uia_text: Option<String>,
    /// Backspace key presses accumulated in this session.
    #[serde(default)]
    pub backspace_count: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Activity {
    pub id: Option<i64>,
    pub day: String,
    pub started_at: DateTime<Utc>,
    pub ended_at: DateTime<Utc>,
    pub app_name: String,
    pub window_title: String,
    pub seconds: u32,
    /// Latest UIA visible-text snapshot for this segment (optional).
    #[serde(default)]
    pub uia_snapshot: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Summary {
    pub day: String,
    pub content: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DayStats {
    pub active_seconds: u32,
    pub session_count: u32,
    pub char_count: u32,
    pub pending_chars: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum FactCategory {
    Project,
    Tool,
    Preference,
    Topic,
    Schedule,
    Routine,
}

impl FactCategory {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Project => "project",
            Self::Tool => "tool",
            Self::Preference => "preference",
            Self::Topic => "topic",
            Self::Schedule => "schedule",
            Self::Routine => "routine",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "project" => Some(Self::Project),
            "tool" => Some(Self::Tool),
            "preference" => Some(Self::Preference),
            "topic" => Some(Self::Topic),
            "schedule" => Some(Self::Schedule),
            "routine" => Some(Self::Routine),
            _ => None,
        }
    }

    /// Singleton categories: a new fact with the same predicate supersedes
    /// older facts holding a different object (e.g. "正在做项目" can only
    /// point at the current project).
    pub fn is_singleton(self) -> bool {
        matches!(self, Self::Project | Self::Tool | Self::Preference)
    }
}

/// Bitemporal subject-predicate-object fact about the user.
/// Superseded facts are marked `invalid_at` instead of deleted.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Fact {
    pub id: Option<i64>,
    pub subject: String,
    pub predicate: String,
    pub object: String,
    pub category: FactCategory,
    pub confidence: f32,
    pub observations: i64,
    pub valid_at: DateTime<Utc>,
    pub invalid_at: Option<DateTime<Utc>>,
    pub source_day: String,
    pub created_at: DateTime<Utc>,
}

impl Fact {
    /// Human-readable single-sentence rendering for UI and prompts.
    pub fn statement(&self) -> String {
        format!("{} {} {}", self.subject, self.predicate, self.object)
    }
}

/// Foreground time per (app, window) aggregate with the richest UIA snapshot.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ActivityAgg {
    pub app_name: String,
    pub window_title: String,
    pub seconds: i64,
    pub uia_snapshot: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum KeyEventKind {
    Char(char),
    Space,
    Enter,
    Backspace,
    Tab,
    Paste,
    Copy,
    ImeComposition,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum FlowEventKind {
    Copy,
    Paste,
}

impl FlowEventKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Copy => "copy",
            Self::Paste => "paste",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "copy" => Some(Self::Copy),
            "paste" => Some(Self::Paste),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FlowEvent {
    pub id: Option<i64>,
    pub day: String,
    pub at: DateTime<Utc>,
    pub kind: FlowEventKind,
    pub app_name: String,
    pub window_title: String,
    pub content_preview: String,
    pub char_len: usize,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TaskUnit {
    pub id: Option<i64>,
    pub day: String,
    pub started_at: DateTime<Utc>,
    pub ended_at: DateTime<Utc>,
    pub name: String,
    pub goal_guess: String,
    pub app_chain: String,
    pub hesitation_score: f32,
    pub confidence: f32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct KeyEvent {
    pub at: DateTime<Utc>,
    pub kind: KeyEventKind,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WindowSample {
    pub at: DateTime<Utc>,
    pub app_name: String,
    pub window_title: String,
}

/// Fact candidate extracted by the LLM before being merged into storage.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CandidateFact {
    pub subject: String,
    pub predicate: String,
    pub object: String,
    pub category: FactCategory,
    pub confidence: f32,
}
