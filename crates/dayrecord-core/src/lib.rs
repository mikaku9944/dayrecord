#![forbid(unsafe_code)]

pub mod connect;
pub mod consolidation;
pub mod context;
pub mod domain;
pub mod error;
pub mod export;
pub mod models;
pub mod paths;
pub mod patterns;
pub mod ports;
pub mod prompt;
pub mod redact;
pub mod summary;

pub use error::CoreError;
