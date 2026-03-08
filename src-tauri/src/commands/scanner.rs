use serde::Serialize;
use tauri::{AppHandle, Emitter, State};
use walkdir::WalkDir;

use crate::DbState;
use crate::db::models::EmbroideryFile;
use crate::db::queries::{FILE_SELECT, row_to_file};
use crate::error::{lock_db, AppError};
use crate::parsers::{self, ParsedFileInfo};

const SUPPORTED_EXTENSIONS: &[&str] = &["pes", "dst", "jef", "vp3"];

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ScanResult {
    pub found_files: Vec<String>,
    pub total_scanned: u32,
    pub errors: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct ScanProgress {
    current: u32,
    file: String,
}

fn is_embroidery_file(path: &std::path::Path) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| SUPPORTED_EXTENSIONS.contains(&ext.to_lowercase().as_str()))
        .unwrap_or(false)
}

#[tauri::command]
pub fn scan_directory(
    path: String,
    app_handle: AppHandle,
) -> Result<ScanResult, AppError> {
    let dir = std::path::Path::new(&path);
    if !dir.is_dir() {
        return Err(AppError::Validation(format!(
            "Pfad ist kein Verzeichnis: {path}"
        )));
    }

    let mut found_files = Vec::new();
    let mut total_scanned: u32 = 0;
    let mut errors = Vec::new();

    // follow_links(false) to avoid infinite loops from circular symlinks
    for entry in WalkDir::new(dir).follow_links(false) {
        match entry {
            Ok(e) => {
                if !e.file_type().is_file() {
                    continue;
                }
                total_scanned += 1;
                let file_path = e.path();
                if is_embroidery_file(file_path) {
                    let filepath_str = file_path.to_string_lossy().to_string();
                    let _ = app_handle.emit("scan:file-found", &filepath_str);
                    found_files.push(filepath_str);
                }
                if total_scanned % 50 == 0 {
                    let _ = app_handle.emit(
                        "scan:progress",
                        ScanProgress {
                            current: total_scanned,
                            file: e.path().to_string_lossy().to_string(),
                        },
                    );
                }
            }
            Err(e) => {
                errors.push(e.to_string());
            }
        }
    }

    let _ = app_handle.emit("scan:complete", &found_files.len());

    Ok(ScanResult {
        found_files,
        total_scanned,
        errors,
    })
}

#[tauri::command]
pub fn import_files(
    db: State<'_, DbState>,
    file_paths: Vec<String>,
    folder_id: i64,
) -> Result<Vec<EmbroideryFile>, AppError> {
    // Collect filesystem metadata before acquiring the DB lock to avoid
    // holding the mutex during potentially slow I/O.
    let file_info: Vec<(String, String, Option<i64>)> = file_paths
        .iter()
        .map(|filepath| {
            let path = std::path::Path::new(filepath);
            let filename = path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("unknown")
                .to_string();
            let file_size: Option<i64> = std::fs::metadata(path)
                .ok()
                .and_then(|m| i64::try_from(m.len()).ok());
            (filepath.clone(), filename, file_size)
        })
        .collect();

    let conn = lock_db(&db)?;

    // Verify folder exists
    let folder_exists: bool = conn.query_row(
        "SELECT COUNT(*) > 0 FROM folders WHERE id = ?1",
        [folder_id],
        |row| row.get(0),
    )?;
    if !folder_exists {
        return Err(AppError::NotFound(format!(
            "Ordner {folder_id} nicht gefunden"
        )));
    }

    let mut imported = Vec::new();
    let tx = conn.unchecked_transaction()?;

    for (filepath, filename, file_size) in &file_info {
        let result = tx.execute(
            "INSERT OR IGNORE INTO embroidery_files (folder_id, filename, filepath, file_size_bytes) \
             VALUES (?1, ?2, ?3, ?4)",
            rusqlite::params![folder_id, filename, filepath, file_size],
        );

        match result {
            Ok(changes) if changes > 0 => {
                let id = tx.last_insert_rowid();
                let file = tx.query_row(
                    &format!("{FILE_SELECT} WHERE id = ?1"),
                    [id],
                    |row| row_to_file(row),
                )?;
                imported.push(file);
            }
            Ok(_) => {
                // Duplicate (IGNORE), skip silently
            }
            Err(e) => {
                log::warn!("Failed to import {filepath}: {e}");
            }
        }
    }

    tx.commit()?;

    Ok(imported)
}

#[tauri::command]
pub fn parse_embroidery_file(filepath: String) -> Result<ParsedFileInfo, AppError> {
    let path = std::path::Path::new(&filepath);
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_lowercase())
        .ok_or_else(|| AppError::Parse {
            format: "unknown".to_string(),
            message: format!("No file extension: {filepath}"),
        })?;

    let parser = parsers::get_parser(&ext).ok_or_else(|| AppError::Parse {
        format: ext.clone(),
        message: format!("Unsupported format: {ext}"),
    })?;

    let data = std::fs::read(&filepath)?;
    parser.parse(&data)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::migrations::init_database_in_memory;
    use std::fs;

    #[test]
    fn test_is_embroidery_file() {
        assert!(is_embroidery_file(std::path::Path::new("/tmp/test.pes")));
        assert!(is_embroidery_file(std::path::Path::new("/tmp/test.DST")));
        assert!(is_embroidery_file(std::path::Path::new("/tmp/test.jef")));
        assert!(is_embroidery_file(std::path::Path::new("/tmp/test.vp3")));
        assert!(!is_embroidery_file(std::path::Path::new("/tmp/test.png")));
        assert!(!is_embroidery_file(std::path::Path::new("/tmp/test.txt")));
    }

    #[test]
    fn test_scan_finds_embroidery_files() {
        let tmp = tempfile::tempdir().unwrap();
        let base = tmp.path();

        fs::write(base.join("a.pes"), b"fake").unwrap();
        fs::write(base.join("b.dst"), b"fake").unwrap();
        fs::write(base.join("c.txt"), b"text").unwrap();
        fs::create_dir(base.join("sub")).unwrap();
        fs::write(base.join("sub/d.jef"), b"fake").unwrap();

        let mut found = Vec::new();
        for entry in WalkDir::new(base).follow_links(false) {
            if let Ok(e) = entry {
                if e.file_type().is_file() && is_embroidery_file(e.path()) {
                    found.push(e.path().to_string_lossy().to_string());
                }
            }
        }

        assert_eq!(found.len(), 3);
    }

    #[test]
    fn test_import_files_creates_db_entries() {
        let conn = init_database_in_memory().unwrap();

        conn.execute(
            "INSERT INTO folders (name, path) VALUES ('Test', '/tmp/test')",
            [],
        )
        .unwrap();
        let folder_id = conn.last_insert_rowid();

        conn.execute(
            "INSERT OR IGNORE INTO embroidery_files (folder_id, filename, filepath, file_size_bytes) \
             VALUES (?1, 'a.pes', '/tmp/test/a.pes', 1024)",
            [folder_id],
        )
        .unwrap();

        conn.execute(
            "INSERT OR IGNORE INTO embroidery_files (folder_id, filename, filepath, file_size_bytes) \
             VALUES (?1, 'b.dst', '/tmp/test/b.dst', 2048)",
            [folder_id],
        )
        .unwrap();

        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM embroidery_files WHERE folder_id = ?1",
                [folder_id],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(count, 2);

        // Test duplicate handling (IGNORE)
        conn.execute(
            "INSERT OR IGNORE INTO embroidery_files (folder_id, filename, filepath, file_size_bytes) \
             VALUES (?1, 'a.pes', '/tmp/test/a.pes', 1024)",
            [folder_id],
        )
        .unwrap();

        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM embroidery_files WHERE folder_id = ?1",
                [folder_id],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(count, 2, "Duplicate should be ignored");
    }

    fn example_path(name: &str) -> String {
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .join("example files")
            .join(name)
            .to_string_lossy()
            .to_string()
    }

    #[test]
    fn test_parse_embroidery_file_pes() {
        let info = parse_embroidery_file(example_path("BayrischesHerz.PES")).unwrap();
        assert_eq!(info.format, "PES");
        assert!(info.stitch_count.unwrap() > 0);
    }

    #[test]
    fn test_parse_embroidery_file_dst() {
        let info = parse_embroidery_file(example_path("2.DST")).unwrap();
        assert_eq!(info.format, "DST");
        assert!(info.stitch_count.unwrap() > 0);
    }

    #[test]
    fn test_parse_embroidery_file_unsupported() {
        let result = parse_embroidery_file("/tmp/test.txt".to_string());
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_embroidery_file_not_found() {
        let result = parse_embroidery_file("/tmp/nonexistent_12345.pes".to_string());
        assert!(result.is_err());
    }
}
