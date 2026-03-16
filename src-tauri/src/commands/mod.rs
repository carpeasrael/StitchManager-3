pub mod ai;
pub mod backup;
pub mod batch;
pub mod convert;
pub mod edit;
pub mod files;
pub mod folders;
pub mod migration;
pub mod scanner;
pub mod settings;
pub mod templates;
pub mod thread_colors;
pub mod transfer;
pub mod print;
pub mod projects;
pub mod manufacturing;
pub mod procurement;
pub mod reports;
pub mod versions;
pub mod viewer;

use crate::error::AppError;
use std::path::{Component, Path};

/// Returns `true` if the path contains parent-directory (`..`) components.
pub fn has_traversal(path: &str) -> bool {
    Path::new(path).components().any(|c| matches!(c, Component::ParentDir))
}

/// Reject paths containing parent-directory (`..`) components.
/// Uses `Path::components()` instead of string matching for robustness.
pub fn validate_no_traversal(path: &str) -> Result<(), AppError> {
    if has_traversal(path) {
        return Err(AppError::Validation("Path traversal not allowed".to_string()));
    }
    Ok(())
}
