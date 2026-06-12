use thiserror::Error;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum CoreError {
    #[error("invalid input: {0}")]
    InvalidInput(String),
    #[error("summary parse failed: {0}")]
    SummaryParse(String),
    #[error("consolidation parse failed: {0}")]
    ConsolidationParse(String),
}
