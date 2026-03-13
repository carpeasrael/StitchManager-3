use std::path::{Path, PathBuf};
use std::time::Instant;

use serde::Serialize;
use tauri::{AppHandle, Emitter, State};

use crate::{DbState, ThumbnailState};
use crate::error::{lock_db, AppError};

use super::scanner::{ImportProgressPayload, PreParsedFile, pre_parse_file, persist_parsed_metadata};

/// Result returned by the migration command.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MigrationResult {
    pub folders_created: u32,
    pub files_imported: u32,
    pub files_skipped: u32,
    pub tags_imported: u32,
    pub elapsed_ms: u64,
}

/// A file entry parsed from the 2stitch XML.
struct TwoStitchFile {
    filepath: String,
    file_size: Option<i64>,
    width_mm: Option<f64>,
    height_mm: Option<f64>,
    stitch_count: Option<i32>,
    notes: Option<String>,
    tags: Vec<String>,
    is_favorite: bool,
    content_hash: Option<String>,
    threads: Vec<TwoStitchThread>,
}

struct TwoStitchThread {
    color_hex: String,
    color_name: Option<String>,
    chart: Option<String>,
}

/// Parse the 2stitch XML file and extract all data.
fn parse_2stitch_xml(xml_content: &str) -> Result<(Vec<String>, Vec<TwoStitchFile>), AppError> {
    let doc = roxmltree::Document::parse(xml_content).map_err(|e| {
        AppError::Parse {
            format: "xml".to_string(),
            message: format!("Failed to parse 2stitch XML: {e}"),
        }
    })?;

    let root = doc.root_element();

    // Parse smart_folders
    let mut folders: Vec<String> = Vec::new();
    if let Some(sf_node) = root.children().find(|n| n.has_tag_name("smart_folders")) {
        for string_node in sf_node.children().filter(|n| n.has_tag_name("string")) {
            if let Some(text) = string_node.text() {
                let trimmed = text.trim();
                if !trimmed.is_empty() {
                    folders.push(trimmed.to_string());
                }
            }
        }
    }

    // Parse files
    let mut files: Vec<TwoStitchFile> = Vec::new();
    if let Some(files_node) = root.children().find(|n| n.has_tag_name("files")) {
        for file_node in files_node.children().filter(|n| n.has_tag_name("file")) {
            if let Some(f) = parse_file_node(&file_node) {
                files.push(f);
            }
        }
    }

    Ok((folders, files))
}

fn get_child_text<'a>(node: &'a roxmltree::Node, tag: &str) -> Option<&'a str> {
    node.children()
        .find(|n| n.has_tag_name(tag))
        .and_then(|n| n.text())
        .map(|t| t.trim())
        .filter(|t| !t.is_empty())
}

fn parse_file_node(node: &roxmltree::Node) -> Option<TwoStitchFile> {
    let filepath = get_child_text(node, "absolute_file_path")?.to_string();

    let file_size = get_child_text(node, "file_size")
        .and_then(|s| s.parse::<i64>().ok());

    let (width_mm, height_mm) = node
        .children()
        .find(|n| n.has_tag_name("design_size"))
        .map(|ds| {
            let w = ds.attribute("w").and_then(|v| v.parse::<f64>().ok());
            let h = ds.attribute("h").and_then(|v| v.parse::<f64>().ok());
            (w, h)
        })
        .unwrap_or((None, None));

    let stitch_count = get_child_text(node, "stitch_count")
        .and_then(|s| s.parse::<i32>().ok());

    let notes = get_child_text(node, "notes").map(|s| s.to_string());

    let content_hash = get_child_text(node, "content_hash").map(|s| s.to_string());

    let is_favorite = get_child_text(node, "is_favorite")
        .map(|s| s == "true")
        .unwrap_or(false);

    // Parse tags
    let mut tags = Vec::new();
    if let Some(tags_node) = node.children().find(|n| n.has_tag_name("tags")) {
        for string_node in tags_node.children().filter(|n| n.has_tag_name("string")) {
            if let Some(text) = string_node.text() {
                let trimmed = text.trim();
                if !trimmed.is_empty() {
                    tags.push(trimmed.to_string());
                }
            }
        }
    }

    // Parse threads
    let mut threads = Vec::new();
    if let Some(threads_node) = node.children().find(|n| n.has_tag_name("threads")) {
        for thread_node in threads_node.children().filter(|n| n.has_tag_name("thread")) {
            if let Some(color_hex) = get_child_text(&thread_node, "color") {
                threads.push(TwoStitchThread {
                    color_hex: color_hex.to_string(),
                    color_name: get_child_text(&thread_node, "color_name").map(|s| s.to_string()),
                    chart: get_child_text(&thread_node, "chart").map(|s| s.to_string()),
                });
            }
        }
    }

    Some(TwoStitchFile {
        filepath,
        file_size,
        width_mm,
        height_mm,
        stitch_count,
        notes,
        tags,
        is_favorite,
        content_hash,
        threads,
    })
}

/// Migrate files, folders, metadata, and tags from 2stitch Organizer.
///
/// Reads the 2stitch XML data file, creates folders, imports files with metadata
/// enrichment (notes, tags, thread brand names), and copies preview thumbnails.
#[tauri::command]
pub fn migrate_from_2stitch(
    db: State<'_, DbState>,
    thumb_state: State<'_, ThumbnailState>,
    xml_path: Option<String>,
    app_handle: AppHandle,
) -> Result<MigrationResult, AppError> {
    let start = Instant::now();

    // Resolve XML path (validate user-supplied path)
    let xml_file = match xml_path {
        Some(p) => {
            if p.contains("..") {
                return Err(AppError::Validation("Path traversal not allowed".to_string()));
            }
            PathBuf::from(p)
        }
        None => {
            let home = dirs::home_dir().ok_or_else(|| {
                AppError::Validation("Home-Verzeichnis nicht gefunden".to_string())
            })?;
            home.join("Library/Application Support/code-and-web.de/2stitch Organizer/2stitch-organizer.xml")
        }
    };

    if !xml_file.exists() {
        return Err(AppError::NotFound(format!(
            "2stitch XML nicht gefunden: {}",
            xml_file.display()
        )));
    }

    let previews_dir = xml_file.parent().map(|p| p.join("previews"));

    let xml_content = std::fs::read_to_string(&xml_file)?;
    let (folder_paths, twostitch_files) = parse_2stitch_xml(&xml_content)?;

    let total = twostitch_files.len() as u32;
    let import_start = Instant::now();

    // --- Phase 1: Create folders ---
    let mut folders_created: u32 = 0;
    {
        let conn = lock_db(&db)?;
        for folder_path in &folder_paths {
            let dir = Path::new(folder_path);
            let folder_name = dir
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("Import")
                .to_string();

            let existing: Option<i64> = conn
                .query_row(
                    "SELECT id FROM folders WHERE path = ?1",
                    [folder_path],
                    |row| row.get(0),
                )
                .ok();

            if existing.is_none() {
                conn.execute(
                    "INSERT INTO folders (name, path) VALUES (?1, ?2)",
                    rusqlite::params![folder_name, folder_path],
                )?;
                folders_created += 1;
            }
        }
    }

    // --- Phase 2: Import files ---
    let mut files_imported: u32 = 0;
    let mut files_skipped: u32 = 0;
    let mut tags_imported: u32 = 0;

    // Pre-parse files and collect filesystem metadata outside the DB lock
    struct MigrationFile {
        ts_file: TwoStitchFile,
        pre: PreParsedFile,
    }

    let pre_parsed: Vec<MigrationFile> = twostitch_files
        .into_iter()
        .map(|ts_file| {
            let mut pre = pre_parse_file(&ts_file.filepath);
            // Fall back to 2stitch file_size when file doesn't exist on disk
            if pre.file_size.is_none() {
                pre.file_size = ts_file.file_size;
            }
            MigrationFile { ts_file, pre }
        })
        .collect();

    let conn = lock_db(&db)?;

    // Load all folders for path matching
    let mut stmt = conn.prepare("SELECT id, path FROM folders WHERE path IS NOT NULL")?;
    let mut folders: Vec<(i64, String)> = stmt
        .query_map([], |row| {
            Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?))
        })?
        .filter_map(|r| r.ok())
        .collect();
    drop(stmt);

    let mut thumb_pending: Vec<(i64, String, String, Option<String>)> = Vec::new(); // (id, filepath, ext, content_hash)

    {
        let tx = conn.unchecked_transaction()?;

        for (idx, info) in pre_parsed.iter().enumerate() {
            let current = (idx + 1) as u32;

            // Find best matching folder
            let best_folder = folders
                .iter()
                .filter(|(_, folder_path)| {
                    let fp = Path::new(&info.ts_file.filepath);
                    let dp = Path::new(folder_path);
                    fp.starts_with(dp)
                })
                .max_by_key(|(_, folder_path)| folder_path.len());

            let folder_id = match best_folder {
                Some((id, _)) => *id,
                None => {
                    // No matching folder — create one from file's parent dir
                    let parent = Path::new(&info.ts_file.filepath)
                        .parent()
                        .map(|p| p.to_string_lossy().to_string())
                        .unwrap_or_else(|| "/".to_string());
                    let parent_name = Path::new(&parent)
                        .file_name()
                        .and_then(|n| n.to_str())
                        .unwrap_or("Import")
                        .to_string();

                    // Check if we already created this folder in this transaction
                    let existing: Option<i64> = tx
                        .query_row(
                            "SELECT id FROM folders WHERE path = ?1",
                            [&parent],
                            |row| row.get(0),
                        )
                        .ok();

                    match existing {
                        Some(id) => id,
                        None => {
                            tx.execute(
                                "INSERT INTO folders (name, path) VALUES (?1, ?2)",
                                rusqlite::params![parent_name, parent],
                            )?;
                            let new_id = tx.last_insert_rowid();
                            folders.push((new_id, parent));
                            folders_created += 1;
                            new_id
                        }
                    }
                }
            };

            let status: String;

            let result = tx.execute(
                "INSERT OR IGNORE INTO embroidery_files (folder_id, filename, filepath, file_size_bytes) \
                 VALUES (?1, ?2, ?3, ?4)",
                rusqlite::params![folder_id, info.pre.filename, info.ts_file.filepath, info.pre.file_size],
            );

            match result {
                Ok(changes) if changes > 0 => {
                    let id = tx.last_insert_rowid();

                    // Apply parser metadata, colors, and format record
                    if let Some(pinfo) = &info.pre.parsed {
                        persist_parsed_metadata(&tx, id, pinfo, &info.ts_file.filepath, info.pre.file_size);

                        // Enrich colors with 2stitch brand/name (post-processing)
                        for (cidx, _) in pinfo.colors.iter().enumerate() {
                            if let Some(ts_thread) = info.ts_file.threads.get(cidx) {
                                if ts_thread.color_name.is_some() || ts_thread.chart.is_some() {
                                    let _ = tx.execute(
                                        "UPDATE file_thread_colors SET \
                                         color_name = COALESCE(color_name, ?3), \
                                         brand = COALESCE(brand, ?4) \
                                         WHERE file_id = ?1 AND sort_order = ?2",
                                        rusqlite::params![id, cidx as i32, ts_thread.color_name, ts_thread.chart],
                                    );
                                }
                            }
                        }
                    } else {
                        // No parser data — use 2stitch metadata as fallback
                        if let Err(e) = tx.execute(
                            "UPDATE embroidery_files SET \
                             stitch_count = ?2, width_mm = ?3, height_mm = ?4, \
                             color_count = ?5 \
                             WHERE id = ?1",
                            rusqlite::params![
                                id,
                                info.ts_file.stitch_count,
                                info.ts_file.width_mm,
                                info.ts_file.height_mm,
                                info.ts_file.threads.len() as i32,
                            ],
                        ) {
                            log::warn!("Failed to update 2stitch metadata for {}: {e}", info.ts_file.filepath);
                        }

                        // Insert 2stitch thread colors directly
                        for (cidx, thread) in info.ts_file.threads.iter().enumerate() {
                            if let Err(e) = tx.execute(
                                "INSERT INTO file_thread_colors (file_id, sort_order, color_hex, color_name, brand, is_ai) \
                                 VALUES (?1, ?2, ?3, ?4, ?5, 0)",
                                rusqlite::params![
                                    id,
                                    cidx as i32,
                                    thread.color_hex,
                                    thread.color_name,
                                    thread.chart,
                                ],
                            ) {
                                log::warn!("Failed to insert 2stitch color for {}: {e}", info.ts_file.filepath);
                            }
                        }
                    }

                    // Apply 2stitch notes as description
                    if let Some(ref notes) = info.ts_file.notes {
                        if let Err(e) = tx.execute(
                            "UPDATE embroidery_files SET description = ?2 WHERE id = ?1 AND (description IS NULL OR description = '')",
                            rusqlite::params![id, notes],
                        ) {
                            log::warn!("Failed to set notes for {}: {e}", info.ts_file.filepath);
                        }
                    }

                    // Import tags
                    let mut file_tags = info.ts_file.tags.clone();
                    if info.ts_file.is_favorite {
                        file_tags.push("favorit".to_string());
                    }

                    for tag_name in &file_tags {
                        // Ensure tag exists
                        tx.execute(
                            "INSERT OR IGNORE INTO tags (name) VALUES (?1)",
                            [tag_name],
                        )?;

                        let tag_id: i64 = tx.query_row(
                            "SELECT id FROM tags WHERE name = ?1",
                            [tag_name],
                            |row| row.get(0),
                        )?;

                        if let Err(e) = tx.execute(
                            "INSERT OR IGNORE INTO file_tags (file_id, tag_id) VALUES (?1, ?2)",
                            rusqlite::params![id, tag_id],
                        ) {
                            log::warn!("Failed to insert tag for {}: {e}", info.ts_file.filepath);
                        }

                        tags_imported += 1;
                    }

                    // Queue thumbnail generation
                    if let Some(ext) = &info.pre.ext {
                        thumb_pending.push((
                            id,
                            info.ts_file.filepath.clone(),
                            ext.clone(),
                            info.ts_file.content_hash.clone(),
                        ));
                    }

                    files_imported += 1;
                    status = "ok".to_string();
                }
                Ok(_) => {
                    files_skipped += 1;
                    status = "skipped".to_string();
                }
                Err(e) => {
                    files_skipped += 1;
                    status = format!("error:{e}");
                    log::warn!("Failed to import {}: {e}", info.ts_file.filepath);
                }
            }

            // Emit progress (throttled)
            if current % 10 == 0 || current == total {
                let elapsed = import_start.elapsed().as_millis() as u64;
                let avg_per_file = if current > 0 { elapsed / current as u64 } else { 0 };
                let remaining = total.saturating_sub(current);
                let estimated_remaining_ms = avg_per_file * remaining as u64;

                let _ = app_handle.emit(
                    "import:progress",
                    ImportProgressPayload {
                        current,
                        total,
                        filename: info.pre.filename.clone(),
                        status,
                        elapsed_ms: elapsed,
                        estimated_remaining_ms,
                    },
                );
            }
        }

        tx.commit()?;
    }

    // Drop DB lock before thumbnail generation
    drop(conn);

    // Generate thumbnails — prefer our stitch-rendered thumbnails;
    // fall back to copying 2stitch preview PNG only if our generator fails
    for (id, filepath, ext, content_hash) in &thumb_pending {
        let mut thumb_set = false;

        // Try our stitch-rendered thumbnail first (if file exists on disk)
        if Path::new(filepath).exists() {
            if let Ok(data) = std::fs::read(Path::new(filepath)) {
                match thumb_state.0.generate(*id, &data, ext) {
                    Ok(thumb_path) => {
                        if let Ok(c) = lock_db(&db) {
                            let _ = c.execute(
                                "UPDATE embroidery_files SET thumbnail_path = ?2 WHERE id = ?1",
                                rusqlite::params![id, thumb_path.to_string_lossy().as_ref()],
                            );
                        }
                        thumb_set = true;
                    }
                    Err(e) => {
                        log::warn!("Failed to generate thumbnail for {filepath}: {e}");
                    }
                }
            }
        }

        // Fallback: copy 2stitch preview if our generator failed
        if !thumb_set {
            if let (Some(previews), Some(hash)) = (&previews_dir, content_hash) {
                let preview_path = previews.join(format!("{hash}.png"));
                if preview_path.exists() {
                    let target = thumb_state.0.thumbnail_path(*id);

                    // Ensure target directory exists
                    if let Some(parent) = target.parent() {
                        let _ = std::fs::create_dir_all(parent);
                    }

                    if let Ok(()) = std::fs::copy(&preview_path, &target).map(|_| ()) {
                        if let Ok(c) = lock_db(&db) {
                            let _ = c.execute(
                                "UPDATE embroidery_files SET thumbnail_path = ?2 WHERE id = ?1",
                                rusqlite::params![id, target.to_string_lossy().as_ref()],
                            );
                        }
                    }
                }
            }
        }
    }

    let elapsed_ms = start.elapsed().as_millis() as u64;

    let result = MigrationResult {
        folders_created,
        files_imported,
        files_skipped,
        tags_imported,
        elapsed_ms,
    };


    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_2stitch_xml_basic() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<tostitch_organizer version="1.0">
    <preset_collections/>
    <smart_folders>
        <string>/tmp/test_folder</string>
    </smart_folders>
    <files>
        <file>
            <absolute_file_path>/tmp/test_folder/test.PES</absolute_file_path>
            <file_size>1024</file_size>
            <modification_date>1700000000</modification_date>
            <content_hash>abc123</content_hash>
            <name>TestName</name>
            <design_size h="50.0" w="60.0"/>
            <stitch_count>1000</stitch_count>
            <threads>
                <thread>
                    <color>#ff0000</color>
                    <color_name>Red</color_name>
                    <chart>Janome</chart>
                </thread>
            </threads>
            <notes>Test notes</notes>
            <tags>
                <string>tag1</string>
                <string>tag2</string>
            </tags>
            <is_favorite>true</is_favorite>
        </file>
    </files>
    <all_tags>
        <string>tag1</string>
        <string>tag2</string>
    </all_tags>
</tostitch_organizer>"#;

        let (folders, files) = parse_2stitch_xml(xml).unwrap();

        assert_eq!(folders.len(), 1);
        assert_eq!(folders[0], "/tmp/test_folder");

        assert_eq!(files.len(), 1);
        let f = &files[0];
        assert_eq!(f.filepath, "/tmp/test_folder/test.PES");
        assert_eq!(f.file_size, Some(1024));
        assert_eq!(f.width_mm, Some(60.0));
        assert_eq!(f.height_mm, Some(50.0));
        assert_eq!(f.stitch_count, Some(1000));
        assert_eq!(f.notes, Some("Test notes".to_string()));
        assert_eq!(f.content_hash, Some("abc123".to_string()));
        assert!(f.is_favorite);

        assert_eq!(f.tags.len(), 2);
        assert_eq!(f.tags[0], "tag1");
        assert_eq!(f.tags[1], "tag2");

        assert_eq!(f.threads.len(), 1);
        assert_eq!(f.threads[0].color_hex, "#ff0000");
        assert_eq!(f.threads[0].color_name, Some("Red".to_string()));
        assert_eq!(f.threads[0].chart, Some("Janome".to_string()));
    }

    #[test]
    fn test_parse_2stitch_xml_empty() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<tostitch_organizer version="1.0">
    <preset_collections/>
    <smart_folders/>
    <files/>
    <all_tags/>
</tostitch_organizer>"#;

        let (folders, files) = parse_2stitch_xml(xml).unwrap();
        assert!(folders.is_empty());
        assert!(files.is_empty());
    }

    #[test]
    fn test_parse_2stitch_xml_missing_optional_fields() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<tostitch_organizer version="1.0">
    <preset_collections/>
    <smart_folders/>
    <files>
        <file>
            <absolute_file_path>/tmp/minimal.DST</absolute_file_path>
            <file_size>500</file_size>
            <modification_date>1700000000</modification_date>
            <content_hash>def456</content_hash>
            <name/>
            <design_size h="0" w="0"/>
            <stitch_count>0</stitch_count>
            <threads/>
            <notes/>
            <tags/>
            <is_favorite>false</is_favorite>
        </file>
    </files>
    <all_tags/>
</tostitch_organizer>"#;

        let (_, files) = parse_2stitch_xml(xml).unwrap();
        assert_eq!(files.len(), 1);
        let f = &files[0];
        assert_eq!(f.filepath, "/tmp/minimal.DST");
        assert!(!f.is_favorite);
        assert!(f.tags.is_empty());
        assert!(f.threads.is_empty());
    }

    #[test]
    fn test_parse_real_2stitch_xml() {
        // Test with actual 2stitch data if available on developer machine
        let path = match dirs::home_dir() {
            Some(home) => home.join("Library/Application Support/code-and-web.de/2stitch Organizer/2stitch-organizer.xml"),
            None => return,
        };

        if !path.exists() {
            return;
        }

        let xml = std::fs::read_to_string(&path).unwrap();
        let (folders, files) = parse_2stitch_xml(&xml).unwrap();

        assert!(!files.is_empty(), "Should have parsed at least one file");
        assert!(!folders.is_empty(), "Should have parsed at least one folder");

        // Verify first file has required fields
        let f = &files[0];
        assert!(!f.filepath.is_empty());
    }
}
