pub mod ai;
pub mod audit;
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
pub mod smart_folders;
pub mod statistics;

use crate::error::AppError;
use std::path::{Component, Path, PathBuf};

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

/// Expand a leading `~/` to the user's home directory.
pub fn expand_home(path: &str) -> PathBuf {
    if let Some(rest) = path.strip_prefix("~/") {
        if let Some(home) = dirs::home_dir() {
            return home.join(rest);
        }
    }
    PathBuf::from(path)
}

/// Resolve a path to its canonical form, falling back to the input if the
/// target does not yet exist. Used for ancestor-prefix containment checks.
pub fn canonicalize_or_self(p: &Path) -> PathBuf {
    std::fs::canonicalize(p).unwrap_or_else(|_| p.to_path_buf())
}

/// Verify `child` lives under `ancestor` after canonicalisation.
/// Both paths are canonicalised (falling back to lexical form on failure)
/// before the prefix comparison, which defends against symlink and case-folding
/// surprises on macOS/Windows.
pub fn ensure_under(child: &Path, ancestor: &Path) -> Result<(), AppError> {
    if has_traversal(&child.to_string_lossy()) {
        return Err(AppError::Validation("Path traversal not allowed".into()));
    }
    let canon_child = canonicalize_or_self(child);
    let canon_ancestor = canonicalize_or_self(ancestor);
    if !canon_child.starts_with(&canon_ancestor) {
        return Err(AppError::Validation(format!(
            "Pfad ausserhalb des erlaubten Verzeichnisses: {}",
            child.display()
        )));
    }
    Ok(())
}

/// Read the `library_root` setting from the DB, expanding `~/` if present.
/// Returns `None` when the setting is absent or empty.
pub fn library_root(conn: &rusqlite::Connection) -> Option<PathBuf> {
    conn.query_row::<String, _, _>(
        "SELECT value FROM settings WHERE key = 'library_root'",
        [],
        |row| row.get(0),
    )
    .ok()
    .filter(|v| !v.trim().is_empty())
    .map(|s| expand_home(&s))
}

/// Validate a candidate `library_root` value. Rejects empty strings, root
/// (`/`), the user's bare home directory, and obvious system roots that would
/// expose sensitive content via the viewer/import paths.
pub fn validate_library_root(raw: &str) -> Result<(), AppError> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Err(AppError::Validation("library_root darf nicht leer sein".into()));
    }
    if has_traversal(trimmed) {
        return Err(AppError::Validation("library_root darf keine '..' enthalten".into()));
    }
    let expanded = expand_home(trimmed);
    let canon = canonicalize_or_self(&expanded);
    let canon_str = canon.to_string_lossy();

    let forbidden_eq = ["/", "/home", "/Users", "C:\\", "C:\\Users"];
    if forbidden_eq.iter().any(|f| canon_str == *f) {
        return Err(AppError::Validation(
            "library_root darf nicht das System- oder Benutzerverzeichnis sein".into(),
        ));
    }
    if let Some(home) = dirs::home_dir() {
        if canon == home {
            return Err(AppError::Validation(
                "library_root darf nicht das Heimatverzeichnis selbst sein".into(),
            ));
        }
    }
    Ok(())
}

/// Allow-listed extensions for files attached to embroidery records.
/// Mirrors the mime mapping in `attach_file` plus a couple of read-only types.
pub const ATTACHMENT_EXTENSIONS: &[&str] = &[
    "pdf", "png", "jpg", "jpeg", "txt", "md",
];

/// Allow-listed extensions readable through the in-app viewer.
pub const VIEWER_EXTENSIONS: &[&str] = &[
    "pdf", "png", "jpg", "jpeg", "bmp", "svg",
    "pes", "dst", "jef", "vp3",
];

/// Lower-case file extension or empty string.
pub fn lower_ext(p: &Path) -> String {
    p.extension()
        .and_then(|e| e.to_str())
        .map(|s| s.to_ascii_lowercase())
        .unwrap_or_default()
}
