use std::collections::HashSet;
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, State};

use crate::db::models::EmbroideryFile;
use crate::db::queries::{FILE_SELECT, row_to_file};
use crate::error::{lock_db, AppError};
use crate::DbState;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BatchResult {
    pub total: i64,
    pub success: i64,
    pub failed: i64,
    pub errors: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BatchProgressPayload {
    pub current: i64,
    pub total: i64,
    pub filename: String,
    pub status: String,
}

/// Sanitize a path component by removing path traversal sequences and absolute paths.
/// Strips `..`, leading `/`, and replaces remaining `/` and `\` in individual values.
fn sanitize_path_component(value: &str) -> String {
    value
        .replace("..", "")
        .replace('/', "_")
        .replace('\\', "_")
        .trim()
        .to_string()
}

/// Sanitize the full output of apply_pattern for use as a path.
/// Preserves `/` that come from the pattern itself (for organize patterns like `{theme}/{name}`)
/// but sanitizes each placeholder's resolved value.
fn sanitize_pattern_output(result: &str) -> String {
    // Remove any ".." path components
    let parts: Vec<&str> = result.split('/').collect();
    let cleaned: Vec<&str> = parts.into_iter().filter(|p| *p != ".." && !p.is_empty()).collect();
    let joined = cleaned.join("/");
    // Strip leading / to prevent absolute paths
    joined.strip_prefix('/').unwrap_or(&joined).to_string()
}

/// Apply pattern substitution to a file's metadata.
/// Placeholders: {name}, {theme}, {format}
/// Individual placeholder values are sanitized to prevent path traversal.
pub fn apply_pattern(pattern: &str, file: &EmbroideryFile) -> String {
    let name = sanitize_path_component(file.name.as_deref().unwrap_or("unbenannt"));
    let theme = sanitize_path_component(file.theme.as_deref().unwrap_or("unbekannt"));
    let format = file
        .filename
        .rsplit('.')
        .next()
        .unwrap_or("bin")
        .to_lowercase();

    let result = pattern
        .replace("{name}", &name)
        .replace("{theme}", &theme)
        .replace("{format}", &format);

    sanitize_pattern_output(&result)
}

/// Given a desired path, return a collision-free path by appending `_1`, `_2`, etc.
/// Checks both the filesystem and a set of already-claimed paths from this batch.
/// Counter is capped at 100_000 to prevent infinite loops.
fn dedup_path(
    path: &std::path::Path,
    claimed: &mut HashSet<std::path::PathBuf>,
) -> std::path::PathBuf {
    let mut candidate = path.to_path_buf();
    if !candidate.exists() && !claimed.contains(&candidate) {
        claimed.insert(candidate.clone());
        return candidate;
    }

    let stem = path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("file");
    let ext = path.extension().and_then(|s| s.to_str());
    let parent = path.parent().unwrap_or(std::path::Path::new(""));

    for counter in 1..=100_000u32 {
        candidate = if let Some(e) = ext {
            parent.join(format!("{stem}_{counter}.{e}"))
        } else {
            parent.join(format!("{stem}_{counter}"))
        };
        if !candidate.exists() && !claimed.contains(&candidate) {
            claimed.insert(candidate.clone());
            return candidate;
        }
    }

    // Fallback: return with max counter suffix (extremely unlikely)
    claimed.insert(candidate.clone());
    candidate
}

#[tauri::command]
pub async fn batch_rename(
    db: State<'_, DbState>,
    app_handle: AppHandle,
    file_ids: Vec<i64>,
    pattern: String,
) -> Result<BatchResult, AppError> {
    let total = file_ids.len() as i64;
    let mut success: i64 = 0;
    let mut failed: i64 = 0;
    let mut errors: Vec<String> = Vec::new();
    let pattern_has_format = pattern.contains("{format}");

    // Track claimed target paths within this batch to detect collisions
    let mut claimed: HashSet<std::path::PathBuf> = HashSet::new();

    for (i, file_id) in file_ids.iter().enumerate() {
        let result = (|| -> Result<String, AppError> {
            // Query file data under lock, then drop lock before file I/O.
            // Note: TOCTOU window exists between lock release and re-acquisition
            // for the DB update. Acceptable for a single-user desktop app.
            let file = {
                let conn = lock_db(&db)?;
                conn.query_row(
                    &format!("{FILE_SELECT} WHERE id = ?1"),
                    [file_id],
                    |row| row_to_file(row),
                )
                .map_err(|e| match e {
                    rusqlite::Error::QueryReturnedNoRows => {
                        AppError::NotFound(format!("Datei {file_id} nicht gefunden"))
                    }
                    other => AppError::Database(other),
                })?
            };

            let ext = file
                .filename
                .rsplit('.')
                .next()
                .unwrap_or("");
            let base = apply_pattern(&pattern, &file);

            // If the pattern already includes {format}, don't append the extension again
            let desired_filename = if pattern_has_format || ext.is_empty() {
                base
            } else {
                format!("{base}.{ext}")
            };

            // Build desired filepath, then deduplicate against collisions
            let old_path = std::path::Path::new(&file.filepath);
            let canonical_old = if old_path.exists() {
                old_path.canonicalize()?
            } else {
                old_path.to_path_buf()
            };
            let parent = old_path.parent().unwrap_or(std::path::Path::new(""));
            let desired_path = parent.join(&desired_filename);
            let new_path = dedup_path(&desired_path, &mut claimed);
            let new_filename = new_path
                .file_name()
                .and_then(|s| s.to_str())
                .unwrap_or(&desired_filename)
                .to_string();

            // Rename physical file if it exists and target differs (no lock held)
            let did_rename = if old_path.exists() && canonical_old != new_path {
                std::fs::rename(old_path, &new_path)?;
                true
            } else {
                false
            };

            // Re-acquire lock for DB update — rollback filesystem on failure
            let conn = lock_db(&db)?;
            if let Err(db_err) = conn.execute(
                "UPDATE embroidery_files SET filename = ?1, filepath = ?2, \
                 updated_at = datetime('now') WHERE id = ?3",
                rusqlite::params![new_filename, new_path.to_string_lossy().as_ref(), file_id],
            ) {
                if did_rename {
                    let _ = std::fs::rename(&new_path, old_path);
                }
                claimed.remove(&new_path);
                return Err(AppError::Database(db_err));
            }

            Ok(new_filename)
        })();

        match result {
            Ok(ref new_filename) => {
                success += 1;
                let _ = app_handle.emit(
                    "batch:progress",
                    BatchProgressPayload {
                        current: (i + 1) as i64,
                        total,
                        filename: new_filename.clone(),
                        status: "success".to_string(),
                    },
                );
            }
            Err(e) => {
                failed += 1;
                let msg = format!("Datei {file_id}: {e}");
                errors.push(msg.clone());
                let _ = app_handle.emit(
                    "batch:progress",
                    BatchProgressPayload {
                        current: (i + 1) as i64,
                        total,
                        filename: format!("Datei {file_id}"),
                        status: format!("error: {e}"),
                    },
                );
            }
        }
    }

    Ok(BatchResult {
        total,
        success,
        failed,
        errors,
    })
}

#[tauri::command]
pub async fn batch_organize(
    db: State<'_, DbState>,
    app_handle: AppHandle,
    file_ids: Vec<i64>,
    pattern: String,
) -> Result<BatchResult, AppError> {
    // Read library_root from settings
    let library_root = {
        let conn = lock_db(&db)?;
        conn.query_row(
            "SELECT value FROM settings WHERE key = 'library_root'",
            [],
            |row| row.get::<_, String>(0),
        )
        .unwrap_or_else(|_| "~/Stickdateien".to_string())
    };

    // Expand ~ to home directory
    let base_dir = if library_root.starts_with("~/") {
        if let Some(home) = dirs::home_dir() {
            home.join(&library_root[2..])
        } else {
            std::path::PathBuf::from(&library_root)
        }
    } else {
        std::path::PathBuf::from(&library_root)
    };

    let total = file_ids.len() as i64;
    let mut success: i64 = 0;
    let mut failed: i64 = 0;
    let mut errors: Vec<String> = Vec::new();

    // Track claimed target paths within this batch to detect collisions
    let mut claimed: HashSet<std::path::PathBuf> = HashSet::new();

    // Canonicalize base_dir early — if it doesn't exist, fail fast
    let canonical_base = base_dir.canonicalize().map_err(|e| {
        AppError::Validation(format!(
            "Bibliotheksverzeichnis nicht gefunden: {}: {e}",
            base_dir.display()
        ))
    })?;

    for (i, file_id) in file_ids.iter().enumerate() {
        // Note: folder_id is intentionally not updated — organize is a filesystem-only
        // operation. The file retains its original folder association in the UI.
        let result = (|| -> Result<String, AppError> {
            // Query file data under lock, then drop lock before file I/O.
            // Note: TOCTOU window exists between lock release and re-acquisition
            // for the DB update. Acceptable for a single-user desktop app.
            let file = {
                let conn = lock_db(&db)?;
                conn.query_row(
                    &format!("{FILE_SELECT} WHERE id = ?1"),
                    [file_id],
                    |row| row_to_file(row),
                )
                .map_err(|e| match e {
                    rusqlite::Error::QueryReturnedNoRows => {
                        AppError::NotFound(format!("Datei {file_id} nicht gefunden"))
                    }
                    other => AppError::Database(other),
                })?
            };

            // Build target subdirectory from pattern (sanitized against path traversal)
            let sub_path = apply_pattern(&pattern, &file);
            let target_dir = base_dir.join(&sub_path);

            // Validate target is under base_dir before creating directories
            // Normalize by collecting components to resolve any remaining ".." segments
            let normalized: std::path::PathBuf = target_dir.components().collect();
            if !normalized.starts_with(&canonical_base) {
                return Err(AppError::Validation(
                    "Zielpfad liegt ausserhalb der Bibliothek".into(),
                ));
            }

            // File I/O without holding the DB lock
            std::fs::create_dir_all(&target_dir)?;

            let old_path = std::path::Path::new(&file.filepath);
            let canonical_old = if old_path.exists() {
                old_path.canonicalize()?
            } else {
                old_path.to_path_buf()
            };
            let desired_path = target_dir.join(&file.filename);
            let new_path = dedup_path(&desired_path, &mut claimed);

            // Move physical file if it exists and target differs (no lock held)
            let did_rename = if old_path.exists() && canonical_old != new_path {
                std::fs::rename(old_path, &new_path)?;
                true
            } else {
                false
            };

            // Re-acquire lock for DB update — rollback filesystem on failure
            let new_filename = new_path
                .file_name()
                .and_then(|s| s.to_str())
                .unwrap_or(&file.filename)
                .to_string();

            let conn = lock_db(&db)?;
            if let Err(db_err) = conn.execute(
                "UPDATE embroidery_files SET filename = ?1, filepath = ?2, \
                 updated_at = datetime('now') WHERE id = ?3",
                rusqlite::params![new_filename, new_path.to_string_lossy().as_ref(), file_id],
            ) {
                if did_rename {
                    let _ = std::fs::rename(&new_path, old_path);
                }
                claimed.remove(&new_path);
                return Err(AppError::Database(db_err));
            }

            Ok(new_filename)
        })();

        match result {
            Ok(ref filename) => {
                success += 1;
                let _ = app_handle.emit(
                    "batch:progress",
                    BatchProgressPayload {
                        current: (i + 1) as i64,
                        total,
                        filename: filename.clone(),
                        status: "success".to_string(),
                    },
                );
            }
            Err(e) => {
                failed += 1;
                let msg = format!("Datei {file_id}: {e}");
                errors.push(msg.clone());
                let _ = app_handle.emit(
                    "batch:progress",
                    BatchProgressPayload {
                        current: (i + 1) as i64,
                        total,
                        filename: format!("Datei {file_id}"),
                        status: format!("error: {e}"),
                    },
                );
            }
        }
    }

    Ok(BatchResult {
        total,
        success,
        failed,
        errors,
    })
}

#[tauri::command]
pub async fn batch_export_usb(
    db: State<'_, DbState>,
    app_handle: AppHandle,
    file_ids: Vec<i64>,
    target_path: String,
) -> Result<BatchResult, AppError> {
    let target_dir = std::path::Path::new(&target_path);
    if !target_dir.exists() {
        std::fs::create_dir_all(target_dir)?;
    }

    let total = file_ids.len() as i64;
    let mut success: i64 = 0;
    let mut failed: i64 = 0;
    let mut errors: Vec<String> = Vec::new();

    // Track claimed target paths within this batch to detect collisions
    let mut claimed: HashSet<std::path::PathBuf> = HashSet::new();

    for (i, file_id) in file_ids.iter().enumerate() {
        let result = (|| -> Result<String, AppError> {
            // Query file data under lock, then drop lock before file I/O
            let (filename, filepath): (String, String) = {
                let conn = lock_db(&db)?;
                conn.query_row(
                    "SELECT filename, filepath FROM embroidery_files WHERE id = ?1",
                    [file_id],
                    |row| Ok((row.get(0)?, row.get(1)?)),
                )
                .map_err(|e| match e {
                    rusqlite::Error::QueryReturnedNoRows => {
                        AppError::NotFound(format!("Datei {file_id} nicht gefunden"))
                    }
                    other => AppError::Database(other),
                })?
            };

            // File I/O without holding the DB lock
            let source = std::path::Path::new(&filepath);
            if !source.exists() {
                return Err(AppError::Io(std::io::Error::new(
                    std::io::ErrorKind::NotFound,
                    format!("Quelldatei nicht gefunden: {filepath}"),
                )));
            }

            // Handle filename collisions by appending numeric suffix
            let desired = target_dir.join(&filename);
            let dest = dedup_path(&desired, &mut claimed);

            std::fs::copy(source, &dest)?;

            Ok(filename)
        })();

        match result {
            Ok(ref filename) => {
                success += 1;
                let _ = app_handle.emit(
                    "batch:progress",
                    BatchProgressPayload {
                        current: (i + 1) as i64,
                        total,
                        filename: filename.clone(),
                        status: "success".to_string(),
                    },
                );
            }
            Err(e) => {
                failed += 1;
                let msg = format!("Datei {file_id}: {e}");
                errors.push(msg.clone());
                let _ = app_handle.emit(
                    "batch:progress",
                    BatchProgressPayload {
                        current: (i + 1) as i64,
                        total,
                        filename: format!("Datei {file_id}"),
                        status: format!("error: {e}"),
                    },
                );
            }
        }
    }

    Ok(BatchResult {
        total,
        success,
        failed,
        errors,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::migrations::init_database_in_memory;

    #[test]
    fn test_apply_pattern_basic() {
        let file = EmbroideryFile {
            id: 1,
            folder_id: 1,
            filename: "rose.pes".to_string(),
            filepath: "/test/rose.pes".to_string(),
            name: Some("Rose Design".to_string()),
            theme: Some("Blumen".to_string()),
            description: None,
            license: None,
            width_mm: None,
            height_mm: None,
            stitch_count: None,
            color_count: None,
            file_size_bytes: None,
            thumbnail_path: None,
            design_name: None,
            jump_count: None,
            trim_count: None,
            hoop_width_mm: None,
            hoop_height_mm: None,
            category: None,
            author: None,
            keywords: None,
            comments: None,
            ai_analyzed: false,
            ai_confirmed: false,
            created_at: String::new(),
            updated_at: String::new(),
        };

        assert_eq!(
            apply_pattern("{name}_{theme}", &file),
            "Rose Design_Blumen"
        );
        assert_eq!(
            apply_pattern("{theme}/{name}", &file),
            "Blumen/Rose Design"
        );
        assert_eq!(
            apply_pattern("{name}.{format}", &file),
            "Rose Design.pes"
        );
    }

    #[test]
    fn test_apply_pattern_missing_metadata() {
        let file = EmbroideryFile {
            id: 1,
            folder_id: 1,
            filename: "test.dst".to_string(),
            filepath: "/test/test.dst".to_string(),
            name: None,
            theme: None,
            description: None,
            license: None,
            width_mm: None,
            height_mm: None,
            stitch_count: None,
            color_count: None,
            file_size_bytes: None,
            thumbnail_path: None,
            design_name: None,
            jump_count: None,
            trim_count: None,
            hoop_width_mm: None,
            hoop_height_mm: None,
            category: None,
            author: None,
            keywords: None,
            comments: None,
            ai_analyzed: false,
            ai_confirmed: false,
            created_at: String::new(),
            updated_at: String::new(),
        };

        assert_eq!(
            apply_pattern("{name}_{theme}", &file),
            "unbenannt_unbekannt"
        );
    }

    #[test]
    fn test_apply_pattern_path_traversal_sanitized() {
        let file = EmbroideryFile {
            id: 1,
            folder_id: 1,
            filename: "test.pes".to_string(),
            filepath: "/test/test.pes".to_string(),
            name: Some("../../etc/passwd".to_string()),
            theme: Some("../secrets".to_string()),
            description: None,
            license: None,
            width_mm: None,
            height_mm: None,
            stitch_count: None,
            color_count: None,
            file_size_bytes: None,
            thumbnail_path: None,
            design_name: None,
            jump_count: None,
            trim_count: None,
            hoop_width_mm: None,
            hoop_height_mm: None,
            category: None,
            author: None,
            keywords: None,
            comments: None,
            ai_analyzed: false,
            ai_confirmed: false,
            created_at: String::new(),
            updated_at: String::new(),
        };

        let result = apply_pattern("{theme}/{name}", &file);
        // Path traversal components should be sanitized
        assert!(!result.contains(".."));
        assert!(!result.starts_with('/'));
    }

    #[test]
    fn test_batch_organize_path_construction() {
        let file = EmbroideryFile {
            id: 1,
            folder_id: 1,
            filename: "rose.pes".to_string(),
            filepath: "/test/rose.pes".to_string(),
            name: Some("Rose".to_string()),
            theme: Some("Blumen".to_string()),
            description: None,
            license: None,
            width_mm: None,
            height_mm: None,
            stitch_count: None,
            color_count: None,
            file_size_bytes: None,
            thumbnail_path: None,
            design_name: None,
            jump_count: None,
            trim_count: None,
            hoop_width_mm: None,
            hoop_height_mm: None,
            category: None,
            author: None,
            keywords: None,
            comments: None,
            ai_analyzed: false,
            ai_confirmed: false,
            created_at: String::new(),
            updated_at: String::new(),
        };

        let pattern = "{theme}/{name}";
        let sub_path = apply_pattern(pattern, &file);
        let base = std::path::Path::new("/library");
        let target = base.join(&sub_path);
        assert_eq!(target.to_string_lossy(), "/library/Blumen/Rose");
    }

    #[test]
    fn test_batch_result_serialization() {
        let result = BatchResult {
            total: 5,
            success: 4,
            failed: 1,
            errors: vec!["Datei 3: nicht gefunden".to_string()],
        };
        let json = serde_json::to_string(&result).unwrap();
        assert!(json.contains("\"total\":5"));
        assert!(json.contains("\"success\":4"));
        assert!(json.contains("\"failed\":1"));
    }

    #[test]
    fn test_rename_pattern_db_integration() {
        let conn = init_database_in_memory().unwrap();

        conn.execute(
            "INSERT INTO folders (name, path) VALUES ('Test', '/test')",
            [],
        )
        .unwrap();
        let folder_id = conn.last_insert_rowid();

        conn.execute(
            "INSERT INTO embroidery_files (folder_id, filename, filepath, name, theme) \
             VALUES (?1, 'original.pes', '/test/original.pes', 'Stern', 'Geometrie')",
            [folder_id],
        )
        .unwrap();
        let file_id = conn.last_insert_rowid();

        // Simulate rename: build new name from pattern
        let file = conn
            .query_row(
                &format!("{FILE_SELECT} WHERE id = ?1"),
                [file_id],
                |row| row_to_file(row),
            )
            .unwrap();

        let new_base = apply_pattern("{name}_{theme}", &file);
        let new_filename = format!("{new_base}.pes");
        assert_eq!(new_filename, "Stern_Geometrie.pes");

        // Update in DB
        conn.execute(
            "UPDATE embroidery_files SET filename = ?1, updated_at = datetime('now') WHERE id = ?2",
            rusqlite::params![new_filename, file_id],
        )
        .unwrap();

        let updated_name: String = conn
            .query_row(
                "SELECT filename FROM embroidery_files WHERE id = ?1",
                [file_id],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(updated_name, "Stern_Geometrie.pes");
    }

    #[test]
    fn test_sanitize_path_component() {
        assert_eq!(sanitize_path_component("normal"), "normal");
        assert_eq!(sanitize_path_component("../etc"), "_etc");
        assert_eq!(sanitize_path_component("../../passwd"), "__passwd");
        assert_eq!(sanitize_path_component("foo/bar"), "foo_bar");
        assert_eq!(sanitize_path_component("foo\\bar"), "foo_bar");
    }

    #[test]
    fn test_dedup_path_no_collision() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("unique_file.pes");
        let mut claimed = HashSet::new();
        let result = dedup_path(&path, &mut claimed);
        assert_eq!(result, path);
        assert!(claimed.contains(&result));
    }

    #[test]
    fn test_dedup_path_batch_collision() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("test_dedup.pes");
        let mut claimed = HashSet::new();

        // First claim succeeds with original name
        let first = dedup_path(&path, &mut claimed);
        assert_eq!(first, path);

        // Second claim gets _1 suffix
        let second = dedup_path(&path, &mut claimed);
        assert_eq!(second, tmp.path().join("test_dedup_1.pes"));

        // Third claim gets _2 suffix
        let third = dedup_path(&path, &mut claimed);
        assert_eq!(third, tmp.path().join("test_dedup_2.pes"));
    }

    #[test]
    fn test_dedup_path_no_extension() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("noext");
        let mut claimed = HashSet::new();

        let first = dedup_path(&path, &mut claimed);
        assert_eq!(first, path);

        let second = dedup_path(&path, &mut claimed);
        assert_eq!(second, tmp.path().join("noext_1"));
    }

    #[test]
    fn test_dedup_path_existing_file_on_disk() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("existing.pes");
        // Create the file on disk
        std::fs::write(&path, b"test").unwrap();

        let mut claimed = HashSet::new();
        let result = dedup_path(&path, &mut claimed);
        // Should skip the existing file and get _1 suffix
        assert_eq!(result, tmp.path().join("existing_1.pes"));
    }
}
