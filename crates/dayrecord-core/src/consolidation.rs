//! Fact extraction pipeline: parse LLM output into triple candidates.
//!
//! Merging is done by the repository (`upsert_fact` increments `observations`
//! on subject/predicate/object conflict; `supersede_facts` bitemporally
//! invalidates competing objects for singleton categories).

use crate::models::{CandidateFact, FactCategory};
use crate::CoreError;

pub fn parse_candidate_facts(json: &str) -> Result<Vec<CandidateFact>, CoreError> {
    #[derive(serde::Deserialize)]
    struct Raw {
        subject: String,
        predicate: String,
        object: String,
        category: String,
        confidence: f32,
    }

    let trimmed = json.trim();
    let start = trimmed
        .find('[')
        .ok_or_else(|| CoreError::ConsolidationParse("no json array".into()))?;
    let end = trimmed
        .rfind(']')
        .ok_or_else(|| CoreError::ConsolidationParse("no json array end".into()))?;
    let slice = &trimmed[start..=end];

    let raw: Vec<Raw> =
        serde_json::from_str(slice).map_err(|e| CoreError::ConsolidationParse(e.to_string()))?;

    Ok(raw
        .into_iter()
        .filter_map(|r| {
            let category = FactCategory::parse(r.category.trim())?;
            let subject = r.subject.trim().to_string();
            let predicate = r.predicate.trim().to_string();
            let object = r.object.trim().to_string();
            if predicate.is_empty() || object.is_empty() {
                return None;
            }
            Some(CandidateFact {
                subject: if subject.is_empty() { "用户".into() } else { subject },
                predicate,
                object,
                category,
                confidence: r.confidence.clamp(0.0, 1.0),
            })
        })
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_triple_candidates() {
        let json = r#"Here are facts: [{"subject":"用户","predicate":"正在做项目","object":"DayRecord","category":"project","confidence":0.9}]"#;
        let facts = parse_candidate_facts(json).unwrap();
        assert_eq!(facts.len(), 1);
        assert_eq!(facts[0].category, FactCategory::Project);
        assert_eq!(facts[0].object, "DayRecord");
    }

    #[test]
    fn drops_invalid_category_and_empty_object() {
        let json = r#"[
            {"subject":"用户","predicate":"喜欢","object":"暗色主题","category":"preference","confidence":0.8},
            {"subject":"用户","predicate":"未知","object":"x","category":"bogus","confidence":0.8},
            {"subject":"用户","predicate":"喜欢","object":"  ","category":"preference","confidence":0.8}
        ]"#;
        let facts = parse_candidate_facts(json).unwrap();
        assert_eq!(facts.len(), 1);
    }

    #[test]
    fn clamps_confidence_and_defaults_subject() {
        let json = r#"[{"subject":"","predicate":"使用工具","object":"Cursor","category":"tool","confidence":1.5}]"#;
        let facts = parse_candidate_facts(json).unwrap();
        assert_eq!(facts[0].confidence, 1.0);
        assert_eq!(facts[0].subject, "用户");
    }

    #[test]
    fn singleton_categories() {
        assert!(FactCategory::Project.is_singleton());
        assert!(FactCategory::Tool.is_singleton());
        assert!(FactCategory::Preference.is_singleton());
        assert!(!FactCategory::Topic.is_singleton());
        assert!(!FactCategory::Schedule.is_singleton());
    }
}
