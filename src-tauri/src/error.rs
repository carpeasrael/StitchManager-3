#[derive(Debug, thiserror::Error)]
#[allow(dead_code)] // Some variants scaffolded for future sprints
pub enum AppError {
    #[error("Datenbankfehler: {0}")]
    Database(#[from] rusqlite::Error),

    #[error("Dateifehler: {0}")]
    Io(#[from] std::io::Error),

    #[error("Parserfehler: {format} — {message}")]
    Parse { format: String, message: String },

    #[error("KI-Fehler: {0}")]
    Ai(String),

    #[error("Nicht gefunden: {0}")]
    NotFound(String),

    #[error("Validierungsfehler: {0}")]
    Validation(String),

    #[error("Interner Fehler: {0}")]
    Internal(String),
}

use std::sync::MutexGuard;
use crate::DbState;

/// Helper to lock the database mutex, mapping poison errors to AppError::Internal.
pub fn lock_db(db: &DbState) -> Result<MutexGuard<'_, rusqlite::Connection>, AppError> {
    db.0.lock().map_err(|e| AppError::Internal(format!("Mutex poisoned: {e}")))
}

impl AppError {
    fn error_code(&self) -> &'static str {
        match self {
            AppError::Database(_) => "DATABASE",
            AppError::Io(_) => "IO",
            AppError::Parse { .. } => "PARSE",
            AppError::Ai(_) => "AI",
            AppError::NotFound(_) => "NOT_FOUND",
            AppError::Validation(_) => "VALIDATION",
            AppError::Internal(_) => "INTERNAL",
        }
    }
}

impl serde::Serialize for AppError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut s = serializer.serialize_struct("AppError", 2)?;
        s.serialize_field("code", self.error_code())?;
        s.serialize_field("message", &self.to_string())?;
        s.end()
    }
}
