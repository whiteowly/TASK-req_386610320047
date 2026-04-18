// Library crate for KnowledgeOps — exposes modules for testing.
// The binary crate (main.rs) uses these same modules.

pub mod config;
pub mod models;
pub mod schema;
pub mod services;
pub mod errors;
pub mod audit;
pub mod logging;
pub mod crypto;
pub mod jobs;
pub mod search;
pub mod import_export;
pub mod standardization;
pub mod alerts;
pub mod db_instrumentation;
pub mod api;
