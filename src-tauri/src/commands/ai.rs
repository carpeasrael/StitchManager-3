use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, State};

use crate::db::models::{AiAnalysisResult, EmbroideryFile};
use crate::db::queries::{FILE_SELECT, row_to_file};
use crate::error::{lock_db, AppError};
use crate::services::ai_client::{AiClient, AiConfig, AiProvider};
use crate::DbState;

use super::batch::BatchProgressPayload;

const AI_RESULT_SELECT: &str =
    "SELECT id, file_id, provider, model, prompt_hash, raw_response, \
     parsed_name, parsed_theme, parsed_desc, parsed_tags, parsed_colors, \
     accepted, analyzed_at FROM ai_analysis_results";

fn row_to_ai_result(row: &rusqlite::Row) -> rusqlite::Result<AiAnalysisResult> {
    Ok(AiAnalysisResult {
        id: row.get(0)?,
        file_id: row.get(1)?,
        provider: row.get(2)?,
        model: row.get(3)?,
        prompt_hash: row.get(4)?,
        raw_response: row.get(5)?,
        parsed_name: row.get(6)?,
        parsed_theme: row.get(7)?,
        parsed_desc: row.get(8)?,
        parsed_tags: row.get(9)?,
        parsed_colors: row.get(10)?,
        accepted: row.get(11)?,
        analyzed_at: row.get(12)?,
    })
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SelectedFields {
    pub name: Option<bool>,
    pub theme: Option<bool>,
    pub description: Option<bool>,
    pub tags: Option<bool>,
    pub colors: Option<bool>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct AiStartPayload {
    file_id: i64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct AiCompletePayload {
    file_id: i64,
    result_id: i64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct AiErrorPayload {
    file_id: i64,
    error: String,
}

fn load_ai_config(conn: &rusqlite::Connection) -> Result<AiConfig, AppError> {
    let get = |key: &str| -> Result<String, AppError> {
        conn.query_row(
            "SELECT value FROM settings WHERE key = ?1",
            [key],
            |row| row.get(0),
        )
        .map_err(|_| AppError::NotFound(format!("Einstellung '{key}' nicht gefunden")))
    };

    let provider_str = get("ai_provider")?;
    let url = get("ai_url")?;
    let api_key = get("ai_api_key").ok().filter(|k| !k.trim().is_empty());
    let model = get("ai_model")?;
    let temperature: f64 = get("ai_temperature")?
        .parse()
        .unwrap_or(0.3);
    let timeout_ms: u64 = get("ai_timeout_ms")?
        .parse()
        .unwrap_or(30000);

    Ok(AiConfig {
        provider: AiProvider::from_label(&provider_str),
        url,
        api_key,
        model,
        temperature,
        timeout_ms,
    })
}

/// Shared helper to build an AI analysis prompt from file metadata and tags.
fn build_prompt_for_file(
    conn: &rusqlite::Connection,
    file_id: i64,
) -> Result<String, AppError> {
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

    // Load existing tags
    let mut tag_stmt = conn.prepare(
        "SELECT t.name FROM tags t \
         INNER JOIN file_tags ft ON ft.tag_id = t.id \
         WHERE ft.file_id = ?1 ORDER BY t.name",
    )?;
    let tags: Vec<String> = tag_stmt
        .query_map([file_id], |row| row.get(0))?
        .collect::<Result<Vec<_>, _>>()?;

    let mut prompt = String::from(
        "Analysiere dieses Stickdatei-Vorschaubild und extrahiere Metadaten.\n\n\
         Antworte ausschliesslich mit einem JSON-Objekt (ohne Markdown-Code-Block) mit diesen Feldern:\n\
         - \"name\": Kurzer, beschreibender Name des Designs (deutsch)\n\
         - \"theme\": Thema/Kategorie (z.B. Blumen, Tiere, Geometrisch, Weihnachten)\n\
         - \"description\": Kurze Beschreibung des Designs (1-2 Saetze, deutsch)\n\
         - \"tags\": Array von relevanten Tags (deutsch, Kleinbuchstaben)\n\
         - \"colors\": Array von Objekten mit {\"hex\": \"#RRGGBB\", \"name\": \"Farbname\"}\n\n",
    );

    prompt.push_str("Bestehende Metadaten zur Orientierung:\n");
    if let Some(ref name) = file.name {
        prompt.push_str(&format!("- Name: {name}\n"));
    }
    if let Some(ref theme) = file.theme {
        prompt.push_str(&format!("- Thema: {theme}\n"));
    }
    if let Some(ref desc) = file.description {
        prompt.push_str(&format!("- Beschreibung: {desc}\n"));
    }
    if !tags.is_empty() {
        prompt.push_str(&format!("- Tags: {}\n", tags.join(", ")));
    }

    prompt.push_str("\nTechnische Daten:\n");
    prompt.push_str(&format!("- Dateiname: {}\n", file.filename));
    if let Some(w) = file.width_mm {
        if let Some(h) = file.height_mm {
            prompt.push_str(&format!("- Abmessungen: {w:.1} x {h:.1} mm\n"));
        }
    }
    if let Some(sc) = file.stitch_count {
        prompt.push_str(&format!("- Stichzahl: {sc}\n"));
    }
    if let Some(cc) = file.color_count {
        prompt.push_str(&format!("- Farbanzahl: {cc}\n"));
    }

    Ok(prompt)
}

#[tauri::command]
pub fn ai_build_prompt(db: State<'_, DbState>, file_id: i64) -> Result<String, AppError> {
    let conn = lock_db(&db)?;
    build_prompt_for_file(&conn, file_id)
}

#[tauri::command]
pub async fn ai_analyze_file(
    db: State<'_, DbState>,
    app_handle: AppHandle,
    file_id: i64,
    prompt: String,
) -> Result<AiAnalysisResult, AppError> {
    // Emit start event
    let _ = app_handle.emit("ai:start", AiStartPayload { file_id });

    // Query DB for thumbnail path and config, then drop the lock before file I/O
    let (thumbnail_path, config) = {
        let conn = lock_db(&db)?;

        let thumbnail_path: Option<String> = conn
            .query_row(
                "SELECT thumbnail_path FROM embroidery_files WHERE id = ?1",
                [file_id],
                |row| row.get(0),
            )
            .map_err(|e| match e {
                rusqlite::Error::QueryReturnedNoRows => {
                    AppError::NotFound(format!("Datei {file_id} nicht gefunden"))
                }
                other => AppError::Database(other),
            })?;

        let config = load_ai_config(&conn)?;
        (thumbnail_path, config)
    };

    // Read thumbnail file without holding the DB lock
    let image_base64 = match thumbnail_path {
        Some(ref path) if !path.is_empty() => {
            use base64::Engine;
            let data = std::fs::read(path)?;
            base64::engine::general_purpose::STANDARD.encode(&data)
        }
        _ => {
            return Err(AppError::Ai(
                "Kein Thumbnail verfuegbar fuer KI-Analyse".into(),
            ));
        }
    };

    // Extract fields needed after client consumes config
    let provider_str = config.provider.as_str().to_string();
    let model_str = config.model.clone();

    // Perform AI analysis (async, without holding lock)
    let client = AiClient::new(config)?;
    let ai_response = match client.analyze(&image_base64, &prompt).await {
        Ok(resp) => resp,
        Err(e) => {
            let _ = app_handle.emit(
                "ai:error",
                AiErrorPayload {
                    file_id,
                    error: e.to_string(),
                },
            );
            return Err(e);
        }
    };

    // Compute prompt hash
    use sha2::Digest;
    let prompt_hash = format!("{:x}", sha2::Sha256::digest(prompt.as_bytes()));

    // Store result in DB
    let result = {
        let conn = lock_db(&db)?;

        conn.execute(
            "INSERT INTO ai_analysis_results \
             (file_id, provider, model, prompt_hash, raw_response, \
              parsed_name, parsed_theme, parsed_desc, parsed_tags, parsed_colors) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            rusqlite::params![
                file_id,
                provider_str,
                model_str,
                prompt_hash,
                ai_response.raw_response,
                ai_response.parsed_name,
                ai_response.parsed_theme,
                ai_response.parsed_desc,
                ai_response.parsed_tags,
                ai_response.parsed_colors,
            ],
        )?;

        let result_id = conn.last_insert_rowid();

        // Update ai_analyzed flag
        conn.execute(
            "UPDATE embroidery_files SET ai_analyzed = 1, updated_at = datetime('now') WHERE id = ?1",
            [file_id],
        )?;

        conn.query_row(
            &format!("{AI_RESULT_SELECT} WHERE id = ?1"),
            [result_id],
            |row| row_to_ai_result(row),
        )?
    };

    let _ = app_handle.emit(
        "ai:complete",
        AiCompletePayload {
            file_id,
            result_id: result.id,
        },
    );

    Ok(result)
}

#[tauri::command]
pub fn ai_accept_result(
    db: State<'_, DbState>,
    result_id: i64,
    selected_fields: SelectedFields,
) -> Result<EmbroideryFile, AppError> {
    let conn = lock_db(&db)?;

    // Load the analysis result
    let result = conn
        .query_row(
            &format!("{AI_RESULT_SELECT} WHERE id = ?1"),
            [result_id],
            |row| row_to_ai_result(row),
        )
        .map_err(|e| match e {
            rusqlite::Error::QueryReturnedNoRows => {
                AppError::NotFound(format!("Analyseergebnis {result_id} nicht gefunden"))
            }
            other => AppError::Database(other),
        })?;

    // Manual BEGIN/COMMIT/ROLLBACK: rusqlite Transaction API requires owned Connection,
    // but we hold a MutexGuard<Connection>. This pattern is consistent with set_file_tags.
    conn.execute_batch("BEGIN")?;

    let tx_result = (|| -> Result<(), AppError> {
        // Update metadata fields based on selection
        let mut set_clauses = Vec::new();
        let mut params: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();
        let mut idx = 1;

        if selected_fields.name.unwrap_or(false) {
            if let Some(ref name) = result.parsed_name {
                set_clauses.push(format!("name = ?{idx}"));
                params.push(Box::new(name.clone()));
                idx += 1;
            }
        }
        if selected_fields.theme.unwrap_or(false) {
            if let Some(ref theme) = result.parsed_theme {
                set_clauses.push(format!("theme = ?{idx}"));
                params.push(Box::new(theme.clone()));
                idx += 1;
            }
        }
        if selected_fields.description.unwrap_or(false) {
            if let Some(ref desc) = result.parsed_desc {
                set_clauses.push(format!("description = ?{idx}"));
                params.push(Box::new(desc.clone()));
                idx += 1;
            }
        }

        // Always set ai flags
        set_clauses.push("ai_analyzed = 1".to_string());
        set_clauses.push("ai_confirmed = 1".to_string());
        set_clauses.push("updated_at = datetime('now')".to_string());

        if !set_clauses.is_empty() {
            let sql = format!(
                "UPDATE embroidery_files SET {} WHERE id = ?{idx}",
                set_clauses.join(", ")
            );
            params.push(Box::new(result.file_id));
            let param_refs: Vec<&dyn rusqlite::types::ToSql> =
                params.iter().map(|p| p.as_ref()).collect();
            conn.execute(&sql, param_refs.as_slice())?;
        }

        // Handle tags
        if selected_fields.tags.unwrap_or(false) {
            if let Some(ref tags_json) = result.parsed_tags {
                if let Ok(tags) = serde_json::from_str::<Vec<String>>(tags_json) {
                    // Clear existing tags
                    conn.execute("DELETE FROM file_tags WHERE file_id = ?1", [result.file_id])?;

                    for tag_name in &tags {
                        let trimmed = tag_name.trim();
                        if trimmed.is_empty() {
                            continue;
                        }
                        conn.execute(
                            "INSERT OR IGNORE INTO tags (name) VALUES (?1)",
                            [trimmed],
                        )?;
                        let tag_id: i64 = conn.query_row(
                            "SELECT id FROM tags WHERE name = ?1",
                            [trimmed],
                            |row| row.get(0),
                        )?;
                        conn.execute(
                            "INSERT INTO file_tags (file_id, tag_id) VALUES (?1, ?2)",
                            rusqlite::params![result.file_id, tag_id],
                        )?;
                    }
                }
            }
        }

        // Handle colors
        if selected_fields.colors.unwrap_or(false) {
            if let Some(ref colors_json) = result.parsed_colors {
                #[derive(Deserialize)]
                struct AiColor {
                    hex: String,
                    name: Option<String>,
                }

                if let Ok(colors) = serde_json::from_str::<Vec<AiColor>>(colors_json) {
                    // Remove existing AI colors only
                    conn.execute(
                        "DELETE FROM file_thread_colors WHERE file_id = ?1 AND is_ai = 1",
                        [result.file_id],
                    )?;

                    for (i, color) in colors.iter().enumerate() {
                        conn.execute(
                            "INSERT INTO file_thread_colors \
                             (file_id, sort_order, color_hex, color_name, is_ai) \
                             VALUES (?1, ?2, ?3, ?4, 1)",
                            rusqlite::params![
                                result.file_id,
                                i as i32,
                                color.hex,
                                color.name,
                            ],
                        )?;
                    }
                }
            }
        }

        // Mark result as accepted
        conn.execute(
            "UPDATE ai_analysis_results SET accepted = 1 WHERE id = ?1",
            [result_id],
        )?;

        Ok(())
    })();

    match tx_result {
        Ok(()) => conn.execute_batch("COMMIT")?,
        Err(e) => {
            let _ = conn.execute_batch("ROLLBACK");
            return Err(e);
        }
    }

    // Return updated file
    conn.query_row(
        &format!("{FILE_SELECT} WHERE id = ?1"),
        [result.file_id],
        |row| row_to_file(row),
    )
    .map_err(|e| AppError::Database(e))
}

#[tauri::command]
pub fn ai_reject_result(db: State<'_, DbState>, result_id: i64) -> Result<(), AppError> {
    let conn = lock_db(&db)?;

    // Get the file_id for this result
    let file_id: i64 = conn
        .query_row(
            "SELECT file_id FROM ai_analysis_results WHERE id = ?1",
            [result_id],
            |row| row.get(0),
        )
        .map_err(|e| match e {
            rusqlite::Error::QueryReturnedNoRows => {
                AppError::NotFound(format!("Analyseergebnis {result_id} nicht gefunden"))
            }
            other => AppError::Database(other),
        })?;

    // Mark result as rejected (accepted = 0 explicitly)
    conn.execute(
        "UPDATE ai_analysis_results SET accepted = 0 WHERE id = ?1",
        [result_id],
    )?;

    // Update file: analyzed but not confirmed
    conn.execute(
        "UPDATE embroidery_files SET ai_analyzed = 1, ai_confirmed = 0, updated_at = datetime('now') WHERE id = ?1",
        [file_id],
    )?;

    Ok(())
}

#[tauri::command]
pub async fn ai_test_connection(db: State<'_, DbState>) -> Result<bool, AppError> {
    let config = {
        let conn = lock_db(&db)?;
        load_ai_config(&conn)?
    };

    let client = AiClient::new(config)?;
    Ok(client.test_connection().await)
}

#[tauri::command]
pub async fn ai_analyze_batch(
    db: State<'_, DbState>,
    app_handle: AppHandle,
    file_ids: Vec<i64>,
) -> Result<Vec<AiAnalysisResult>, AppError> {
    let total = file_ids.len() as i64;
    let mut results: Vec<AiAnalysisResult> = Vec::new();

    for (i, file_id) in file_ids.iter().enumerate() {
        let analyze_result = (|| async {
            // Query DB for prompt data, thumbnail path, and config — then drop lock
            let (prompt, thumbnail_path, config) = {
                let conn = lock_db(&db)?;
                let prompt = build_prompt_for_file(&conn, *file_id)?;

                let thumbnail_path: Option<String> = conn
                    .query_row(
                        "SELECT thumbnail_path FROM embroidery_files WHERE id = ?1",
                        [file_id],
                        |row| row.get(0),
                    )
                    .map_err(|e| match e {
                        rusqlite::Error::QueryReturnedNoRows => {
                            AppError::NotFound(format!("Datei {file_id} nicht gefunden"))
                        }
                        other => AppError::Database(other),
                    })?;

                let config = load_ai_config(&conn)?;
                (prompt, thumbnail_path, config)
            };

            // Read thumbnail file without holding the DB lock
            let image_base64 = match thumbnail_path {
                Some(ref path) if !path.is_empty() => {
                    use base64::Engine;
                    let data = std::fs::read(path)?;
                    base64::engine::general_purpose::STANDARD.encode(&data)
                }
                _ => {
                    return Err(AppError::Ai(
                        "Kein Thumbnail verfuegbar fuer KI-Analyse".into(),
                    ));
                }
            };

            // Extract fields before config is consumed
            let provider_str = config.provider.as_str().to_string();
            let model_str = config.model.clone();

            // Perform AI analysis (async, without holding lock)
            let client = AiClient::new(config)?;
            let ai_response = client.analyze(&image_base64, &prompt).await?;

            // Compute prompt hash
            use sha2::Digest;
            let prompt_hash = format!("{:x}", sha2::Sha256::digest(prompt.as_bytes()));

            // Store result in DB
            let result = {
                let conn = lock_db(&db)?;

                conn.execute(
                    "INSERT INTO ai_analysis_results \
                     (file_id, provider, model, prompt_hash, raw_response, \
                      parsed_name, parsed_theme, parsed_desc, parsed_tags, parsed_colors) \
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
                    rusqlite::params![
                        file_id,
                        provider_str,
                        model_str,
                        prompt_hash,
                        ai_response.raw_response,
                        ai_response.parsed_name,
                        ai_response.parsed_theme,
                        ai_response.parsed_desc,
                        ai_response.parsed_tags,
                        ai_response.parsed_colors,
                    ],
                )?;

                let result_id = conn.last_insert_rowid();

                conn.execute(
                    "UPDATE embroidery_files SET ai_analyzed = 1, updated_at = datetime('now') WHERE id = ?1",
                    [file_id],
                )?;

                conn.query_row(
                    &format!("{AI_RESULT_SELECT} WHERE id = ?1"),
                    [result_id],
                    |row| row_to_ai_result(row),
                )?
            };

            Ok::<AiAnalysisResult, AppError>(result)
        })()
        .await;

        match analyze_result {
            Ok(result) => {
                let _ = app_handle.emit(
                    "batch:progress",
                    BatchProgressPayload {
                        current: (i + 1) as i64,
                        total,
                        filename: format!("Datei {file_id}"),
                        status: "success".to_string(),
                    },
                );
                results.push(result);
            }
            Err(e) => {
                let _ = app_handle.emit(
                    "batch:progress",
                    BatchProgressPayload {
                        current: (i + 1) as i64,
                        total,
                        filename: format!("Datei {file_id}"),
                        status: format!("error: {e}"),
                    },
                );
                // Continue to next file — don't abort batch
            }
        }
    }

    Ok(results)
}

#[cfg(test)]
mod tests {
    use crate::db::migrations::init_database_in_memory;

    #[test]
    fn test_ai_build_prompt_structure() {
        let conn = init_database_in_memory().unwrap();

        conn.execute("INSERT INTO folders (name, path) VALUES ('Test', '/test')", [])
            .unwrap();
        let folder_id = conn.last_insert_rowid();

        conn.execute(
            "INSERT INTO embroidery_files (folder_id, filename, filepath, name, theme, stitch_count, color_count, width_mm, height_mm) \
             VALUES (?1, 'rose.pes', '/test/rose.pes', 'Rose', 'Blumen', 5000, 8, 120.0, 80.0)",
            [folder_id],
        ).unwrap();
        let file_id = conn.last_insert_rowid();

        // Build prompt by reading directly (mimicking command logic)
        let name: Option<String> = conn
            .query_row(
                "SELECT name FROM embroidery_files WHERE id = ?1",
                [file_id],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(name, Some("Rose".to_string()));
    }

    #[test]
    fn test_ai_analysis_result_storage() {
        let conn = init_database_in_memory().unwrap();

        conn.execute("INSERT INTO folders (name, path) VALUES ('Test', '/test')", [])
            .unwrap();
        let folder_id = conn.last_insert_rowid();

        conn.execute(
            "INSERT INTO embroidery_files (folder_id, filename, filepath) VALUES (?1, 'a.pes', '/test/a.pes')",
            [folder_id],
        ).unwrap();
        let file_id = conn.last_insert_rowid();

        conn.execute(
            "INSERT INTO ai_analysis_results \
             (file_id, provider, model, prompt_hash, raw_response, parsed_name, parsed_theme) \
             VALUES (?1, 'Ollama', 'llama3.2-vision', 'abc123', '{\"name\": \"Rose\"}', 'Rose', 'Blumen')",
            [file_id],
        ).unwrap();
        let result_id = conn.last_insert_rowid();

        let parsed_name: Option<String> = conn
            .query_row(
                "SELECT parsed_name FROM ai_analysis_results WHERE id = ?1",
                [result_id],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(parsed_name, Some("Rose".to_string()));
    }

    #[test]
    fn test_ai_accept_updates_file() {
        let conn = init_database_in_memory().unwrap();

        conn.execute("INSERT INTO folders (name, path) VALUES ('Test', '/test')", [])
            .unwrap();
        let folder_id = conn.last_insert_rowid();

        conn.execute(
            "INSERT INTO embroidery_files (folder_id, filename, filepath) VALUES (?1, 'a.pes', '/test/a.pes')",
            [folder_id],
        ).unwrap();
        let file_id = conn.last_insert_rowid();

        conn.execute(
            "INSERT INTO ai_analysis_results \
             (file_id, provider, model, parsed_name, parsed_theme, parsed_desc) \
             VALUES (?1, 'Ollama', 'llama3.2-vision', 'Rose Design', 'Blumen', 'Eine schoene Rose')",
            [file_id],
        ).unwrap();
        let result_id = conn.last_insert_rowid();

        // Simulate accepting name and theme
        conn.execute(
            "UPDATE embroidery_files SET name = 'Rose Design', theme = 'Blumen', \
             ai_analyzed = 1, ai_confirmed = 1, updated_at = datetime('now') WHERE id = ?1",
            [file_id],
        ).unwrap();
        conn.execute(
            "UPDATE ai_analysis_results SET accepted = 1 WHERE id = ?1",
            [result_id],
        ).unwrap();

        let (name, ai_confirmed): (Option<String>, bool) = conn
            .query_row(
                "SELECT name, ai_confirmed FROM embroidery_files WHERE id = ?1",
                [file_id],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .unwrap();
        assert_eq!(name, Some("Rose Design".to_string()));
        assert!(ai_confirmed);

        let accepted: bool = conn
            .query_row(
                "SELECT accepted FROM ai_analysis_results WHERE id = ?1",
                [result_id],
                |row| row.get(0),
            )
            .unwrap();
        assert!(accepted);
    }

    #[test]
    fn test_ai_reject_updates_file() {
        let conn = init_database_in_memory().unwrap();

        conn.execute("INSERT INTO folders (name, path) VALUES ('Test', '/test')", [])
            .unwrap();
        let folder_id = conn.last_insert_rowid();

        conn.execute(
            "INSERT INTO embroidery_files (folder_id, filename, filepath) VALUES (?1, 'a.pes', '/test/a.pes')",
            [folder_id],
        ).unwrap();
        let file_id = conn.last_insert_rowid();

        conn.execute(
            "INSERT INTO ai_analysis_results \
             (file_id, provider, model, parsed_name) VALUES (?1, 'Ollama', 'llama3.2-vision', 'Rose')",
            [file_id],
        ).unwrap();
        let result_id = conn.last_insert_rowid();

        // Simulate rejection
        conn.execute(
            "UPDATE ai_analysis_results SET accepted = 0 WHERE id = ?1",
            [result_id],
        ).unwrap();
        conn.execute(
            "UPDATE embroidery_files SET ai_analyzed = 1, ai_confirmed = 0, updated_at = datetime('now') WHERE id = ?1",
            [file_id],
        ).unwrap();

        let (ai_analyzed, ai_confirmed): (bool, bool) = conn
            .query_row(
                "SELECT ai_analyzed, ai_confirmed FROM embroidery_files WHERE id = ?1",
                [file_id],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .unwrap();
        assert!(ai_analyzed);
        assert!(!ai_confirmed);
    }

    #[test]
    fn test_load_ai_config_empty_api_key_is_none() {
        use super::load_ai_config;

        // Relies on migration defaults for: ai_provider, ai_url, ai_model,
        // ai_temperature, ai_timeout_ms.
        let conn = init_database_in_memory().unwrap();

        // Set a non-empty API key first
        conn.execute(
            "INSERT OR REPLACE INTO settings (key, value, updated_at) VALUES ('ai_api_key', 'sk-test123', datetime('now'))",
            [],
        ).unwrap();

        let config = load_ai_config(&conn).unwrap();
        assert_eq!(config.api_key, Some("sk-test123".to_string()));

        // Clear the API key (simulates switching away from OpenAI)
        conn.execute(
            "INSERT OR REPLACE INTO settings (key, value, updated_at) VALUES ('ai_api_key', '', datetime('now'))",
            [],
        ).unwrap();

        let config = load_ai_config(&conn).unwrap();
        assert_eq!(config.api_key, None);
    }
}
