use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager, State};

use crate::DbState;
use crate::error::{lock_db, AppError};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TemplateInfo {
    pub id: String,
    pub name: String,
    pub category: String,
    pub description: String,
    pub filename: String,
}

/// List all available templates from the app's template directory.
#[tauri::command]
pub fn list_templates(app: AppHandle) -> Result<Vec<TemplateInfo>, AppError> {
    let template_dir = get_template_dir(&app)?;

    // Read manifest.json if it exists
    let manifest_path = template_dir.join("manifest.json");
    if manifest_path.exists() {
        let data = std::fs::read_to_string(&manifest_path)?;
        let templates: Vec<TemplateInfo> = serde_json::from_str(&data)
            .map_err(|e| AppError::Internal(format!("Template-Manifest ungueltig: {e}")))?;
        // Filter to only templates whose files exist and sanitize filenames
        let existing: Vec<TemplateInfo> = templates.into_iter()
            .filter(|t| {
                // Prevent path traversal in manifest filenames
                let name = &t.filename;
                !name.contains("..") && !name.starts_with('/') && !name.starts_with('\\')
                    && template_dir.join(name).exists()
            })
            .collect();
        return Ok(existing);
    }

    // Fallback: scan directory for embroidery files
    let mut templates = Vec::new();
    if template_dir.exists() {
        for entry in std::fs::read_dir(&template_dir)? {
            let entry = entry?;
            let path = entry.path();
            if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                if matches!(ext.to_lowercase().as_str(), "pes" | "dst" | "jef" | "vp3") {
                    let stem = path.file_stem()
                        .and_then(|s| s.to_str())
                        .unwrap_or("Vorlage")
                        .to_string();
                    templates.push(TemplateInfo {
                        id: stem.clone(),
                        name: stem,
                        category: "Allgemein".to_string(),
                        description: String::new(),
                        filename: entry.file_name().to_string_lossy().to_string(),
                    });
                }
            }
        }
    }

    Ok(templates)
}

/// Copy a template file into the user's library folder.
#[tauri::command]
pub fn instantiate_template(
    app: AppHandle,
    db: State<'_, DbState>,
    template_id: String,
    folder_id: i64,
    name: String,
) -> Result<String, AppError> {
    let template_dir = get_template_dir(&app)?;

    // Find the template file
    let templates = list_templates(app)?;
    let template = templates.iter()
        .find(|t| t.id == template_id)
        .ok_or_else(|| AppError::NotFound(format!("Vorlage '{template_id}' nicht gefunden")))?;

    let src_path = template_dir.join(&template.filename);
    if !src_path.exists() {
        return Err(AppError::NotFound("Vorlagendatei nicht gefunden".into()));
    }

    // Get the target folder path
    let folder_path: String = {
        let conn = lock_db(&db)?;
        conn.query_row(
            "SELECT path FROM folders WHERE id = ?1",
            [folder_id],
            |row| row.get(0),
        ).map_err(|e| match e {
            rusqlite::Error::QueryReturnedNoRows => {
                AppError::NotFound(format!("Ordner {folder_id} nicht gefunden"))
            }
            other => AppError::Database(other),
        })?
    };

    // Build output filename
    let ext = src_path.extension()
        .and_then(|e| e.to_str())
        .unwrap_or("pes");
    let safe_name = name.replace(['/', '\\', '.'], "_");
    let output_filename = format!("{safe_name}.{ext}");
    let output_path = std::path::Path::new(&folder_path).join(&output_filename);

    // Copy
    std::fs::copy(&src_path, &output_path)?;

    Ok(output_path.to_string_lossy().to_string())
}

fn get_template_dir(app: &AppHandle) -> Result<std::path::PathBuf, AppError> {
    let resource_dir = app.path().resource_dir()
        .map_err(|e| AppError::Internal(format!("Resource-Verzeichnis nicht gefunden: {e}")))?;
    Ok(resource_dir.join("templates"))
}
