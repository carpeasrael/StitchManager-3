pub mod migrations;
#[allow(dead_code)] // Models scaffolded for Sprint 2+
pub mod models;

pub use migrations::init_database;
