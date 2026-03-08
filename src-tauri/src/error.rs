#[derive(Debug, thiserror::Error)]
#[allow(dead_code)] // Variants scaffolded for Sprint 2+
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
}

impl serde::Serialize for AppError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}
