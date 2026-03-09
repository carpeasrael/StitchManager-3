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

    for (i, file_id) in file_ids.iter().enumerate() {
        // Note: filesystem rename and DB update are not atomic. If the DB update
        // fails after a successful rename, the file will have its new name on disk
        // but the old name in the database. This is acceptable for batch operations
        // where individual file errors are logged and skipped.
        let result = (|| -> Result<String, AppError> {
            let conn = lock_db(&db)?;

            let file = conn
                .query_row(
                    &format!("{FILE_SELECT} WHERE id = ?1"),
                    [file_id],
                    |row| row_to_file(row),
                )
                .map_err(|e| match e {
                    rusqlite::Error::QueryReturnedNoRows => {
                        AppError::NotFound(format!("Datei {file_id} nicht gefunden"))
                    }
                    other => AppError::Database(other),
                })?;

            let ext = file
                .filename
                .rsplit('.')
                .next()
                .unwrap_or("");
            let base = apply_pattern(&pattern, &file);

            // If the pattern already includes {format}, don't append the extension again
            let new_filename = if pattern_has_format || ext.is_empty() {
                base
            } else {
                format!("{base}.{ext}")
            };

            // Build new filepath
            let old_path = std::path::Path::new(&file.filepath);
            let parent = old_path.parent().unwrap_or(std::path::Path::new(""));
            let new_path = parent.join(&new_filename);

            // Rename physical file if it exists
            if old_path.exists() {
                std::fs::rename(old_path, &new_path)?;
            }

            // Update DB
            conn.execute(
                "UPDATE embroidery_files SET filename = ?1, filepath = ?2, \
                 updated_at = datetime('now') WHERE id = ?3",
                rusqlite::params![new_filename, new_path.to_string_lossy().as_ref(), file_id],
            )?;

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

    for (i, file_id) in file_ids.iter().enumerate() {
        // Note: filesystem move and DB update are not atomic. See batch_rename comment.
        // Note: folder_id is intentionally not updated — organize is a filesystem-only
        // operation. The file retains its original folder association in the UI.
        let result = (|| -> Result<String, AppError> {
            let conn = lock_db(&db)?;

            let file = conn
                .query_row(
                    &format!("{FILE_SELECT} WHERE id = ?1"),
                    [file_id],
                    |row| row_to_file(row),
                )
                .map_err(|e| match e {
                    rusqlite::Error::QueryReturnedNoRows => {
                        AppError::NotFound(format!("Datei {file_id} nicht gefunden"))
                    }
                    other => AppError::Database(other),
                })?;

            // Build target subdirectory from pattern (sanitized against path traversal)
            let sub_path = apply_pattern(&pattern, &file);
            let target_dir = base_dir.join(&sub_path);

            // Verify the resolved target is still under base_dir
            let canonical_base = base_dir.canonicalize().unwrap_or_else(|_| base_dir.clone());
            std::fs::create_dir_all(&target_dir)?;
            let canonical_target = target_dir.canonicalize()?;
            if !canonical_target.starts_with(&canonical_base) {
                return Err(AppError::Validation(
                    "Zielpfad liegt ausserhalb der Bibliothek".into(),
                ));
            }

            let old_path = std::path::Path::new(&file.filepath);
            let new_path = target_dir.join(&file.filename);

            // Move physical file if it exists and isn't already there
            if old_path.exists() && old_path != new_path {
                std::fs::rename(old_path, &new_path)?;
            }

            // Update DB
            conn.execute(
                "UPDATE embroidery_files SET filepath = ?1, \
                 updated_at = datetime('now') WHERE id = ?2",
                rusqlite::params![new_path.to_string_lossy().as_ref(), file_id],
            )?;

            Ok(file.filename.clone())
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

    for (i, file_id) in file_ids.iter().enumerate() {
        let result = (|| -> Result<String, AppError> {
            let conn = lock_db(&db)?;

            let (filename, filepath): (String, String) = conn
                .query_row(
                    "SELECT filename, filepath FROM embroidery_files WHERE id = ?1",
                    [file_id],
                    |row| Ok((row.get(0)?, row.get(1)?)),
                )
                .map_err(|e| match e {
                    rusqlite::Error::QueryReturnedNoRows => {
                        AppError::NotFound(format!("Datei {file_id} nicht gefunden"))
                    }
                    other => AppError::Database(other),
                })?;

            let source = std::path::Path::new(&filepath);
            if !source.exists() {
                return Err(AppError::Io(std::io::Error::new(
                    std::io::ErrorKind::NotFound,
                    format!("Quelldatei nicht gefunden: {filepath}"),
                )));
            }

            // Handle filename collisions by appending numeric suffix
            let mut dest = target_dir.join(&filename);
            if dest.exists() {
                let stem = std::path::Path::new(&filename)
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or(&filename);
                let ext = std::path::Path::new(&filename)
                    .extension()
                    .and_then(|s| s.to_str())
                    .unwrap_or("");
                let mut counter = 1;
                loop {
                    let candidate = if ext.is_empty() {
                        format!("{stem}_{counter}")
                    } else {
                        format!("{stem}_{counter}.{ext}")
                    };
                    dest = target_dir.join(&candidate);
                    if !dest.exists() {
                        break;
                    }
                    counter += 1;
                }
            }

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
}
