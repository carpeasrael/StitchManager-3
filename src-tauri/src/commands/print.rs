use serde::{Deserialize, Serialize};
use tauri::State;

use crate::error::{lock_db, AppError};
use crate::DbState;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PrinterInfo {
    pub name: String,
    pub display_name: String,
    pub is_default: bool,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PrintSettings {
    pub printer_name: Option<String>,
    pub paper_size: String,
    pub orientation: String,
    pub copies: u32,
    pub scale: f64,
    pub fit_to_page: bool,
    pub page_ranges: Option<String>, // e.g. "1-3,5,8-10"
    pub tile_enabled: bool,
    pub tile_overlap_mm: f64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TileInfo {
    pub source_page: u32,
    pub cols: u32,
    pub rows: u32,
    pub total_tiles: u32,
    pub tile_width_mm: f64,
    pub tile_height_mm: f64,
}

/// Compute tiling layout for a large-format page.
#[tauri::command]
pub fn compute_tiles(
    page_width_mm: f64,
    page_height_mm: f64,
    paper_size: String,
    overlap_mm: f64,
) -> Result<TileInfo, AppError> {
    let (target_w, target_h) = paper_size_mm(&paper_size);
    let overlap = overlap_mm.max(0.0).min(50.0);

    let effective_w = target_w - overlap;
    let effective_h = target_h - overlap;

    if effective_w <= 0.0 || effective_h <= 0.0 {
        return Err(AppError::Validation("Ueberlappung ist zu gross fuer die Papiergroesse".into()));
    }

    let cols = ((page_width_mm - overlap) / effective_w).ceil().max(1.0) as u32;
    let rows = ((page_height_mm - overlap) / effective_h).ceil().max(1.0) as u32;

    Ok(TileInfo {
        source_page: 0,
        cols,
        rows,
        total_tiles: cols * rows,
        tile_width_mm: target_w,
        tile_height_mm: target_h,
    })
}

fn paper_size_mm(size: &str) -> (f64, f64) {
    match size.to_lowercase().as_str() {
        "a4" => (210.0, 297.0),
        "letter" | "us letter" => (215.9, 279.4),
        "a3" => (297.0, 420.0),
        "a2" => (420.0, 594.0),
        "a1" => (594.0, 841.0),
        "a0" => (841.0, 1189.0),
        _ => (210.0, 297.0),
    }
}

/// List available printers on the system.
#[tauri::command]
pub fn get_printers() -> Result<Vec<PrinterInfo>, AppError> {
    #[cfg(target_os = "macos")]
    {
        list_printers_macos()
    }
    #[cfg(target_os = "linux")]
    {
        list_printers_macos() // lpstat works on Linux too
    }
    #[cfg(target_os = "windows")]
    {
        list_printers_windows()
    }
}

#[cfg(any(target_os = "macos", target_os = "linux"))]
fn list_printers_macos() -> Result<Vec<PrinterInfo>, AppError> {
    let output = std::process::Command::new("lpstat")
        .args(["-p", "-d"])
        .output()
        .map_err(|e| AppError::Internal(format!("lpstat konnte nicht ausgefuehrt werden: {e}")))?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut printers: Vec<PrinterInfo> = Vec::new();
    let mut default_name = String::new();

    for line in stdout.lines() {
        if let Some(rest) = line.strip_prefix("printer ") {
            // "printer MyPrinter is idle." or "printer MyPrinter disabled since ..."
            if let Some(name_end) = rest.find(" is ").or_else(|| rest.find(" disabled")) {
                let name = rest[..name_end].trim().to_string();
                printers.push(PrinterInfo {
                    display_name: name.replace('_', " "),
                    name,
                    is_default: false,
                });
            }
        } else if let Some(rest) = line.strip_prefix("system default destination: ") {
            default_name = rest.trim().to_string();
        }
    }

    // Mark default printer
    for p in &mut printers {
        if p.name == default_name {
            p.is_default = true;
        }
    }

    Ok(printers)
}

/// Validate print settings to prevent injection via printer_name or page_ranges.
fn validate_print_settings(settings: &PrintSettings) -> Result<(), AppError> {
    // Validate printer name: only allow alphanumeric, hyphens, underscores, spaces, dots
    if let Some(printer) = &settings.printer_name {
        if !printer.chars().all(|c| c.is_alphanumeric() || "-_. ".contains(c)) {
            return Err(AppError::Validation("Ungueltiger Druckername".into()));
        }
    }
    // Validate page ranges: only allow digits, hyphens, commas, spaces
    if let Some(ranges) = &settings.page_ranges {
        if !ranges.chars().all(|c| c.is_ascii_digit() || "-, ".contains(c)) {
            return Err(AppError::Validation("Ungueltige Seitenauswahl".into()));
        }
    }
    // Validate tile overlap
    if settings.tile_enabled && (settings.tile_overlap_mm < 0.0 || settings.tile_overlap_mm > 50.0) {
        return Err(AppError::Validation("Ueberlappung muss zwischen 0 und 50 mm liegen".into()));
    }
    // Validate copies: 1-99
    if settings.copies == 0 || settings.copies > 99 {
        return Err(AppError::Validation("Exemplare muss zwischen 1 und 99 liegen".into()));
    }
    // Validate scale: 0.1 to 5.0
    if !settings.scale.is_finite() || settings.scale < 0.1 || settings.scale > 5.0 {
        return Err(AppError::Validation("Skalierung muss zwischen 10% und 500% liegen".into()));
    }
    Ok(())
}

/// Map paper size name to the media option for lpr.
fn map_paper_size(size: &str) -> &str {
    match size.to_lowercase().as_str() {
        "a4" => "A4",
        "a3" => "A3",
        "letter" | "us letter" => "Letter",
        "legal" => "Legal",
        "a2" => "A2",
        "a1" => "A1",
        "a0" => "A0",
        _ => "A4",
    }
}

/// Print a PDF file directly using the OS print system.
#[tauri::command]
pub async fn print_pdf(
    file_path: String,
    settings: PrintSettings,
) -> Result<(), AppError> {
    super::validate_no_traversal(&file_path)?;
    let path = std::path::Path::new(&file_path);
    if !path.exists() || !path.is_file() {
        return Err(AppError::NotFound(format!(
            "Datei nicht gefunden: {file_path}"
        )));
    }

    // Validate settings
    validate_print_settings(&settings)?;

    #[cfg(any(target_os = "macos", target_os = "linux"))]
    {
        print_file_lpr(&file_path, &settings)
    }
    #[cfg(target_os = "windows")]
    {
        print_file_windows(&file_path, &settings)
    }
}

#[cfg(any(target_os = "macos", target_os = "linux"))]
fn print_file_lpr(path: &str, settings: &PrintSettings) -> Result<(), AppError> {
    let mut cmd = std::process::Command::new("lpr");

    // Printer selection
    if let Some(printer) = &settings.printer_name {
        cmd.arg("-P").arg(printer);
    }

    // Copies
    if settings.copies > 1 {
        cmd.arg("-#").arg(settings.copies.to_string());
    }

    // Paper size
    cmd.arg("-o").arg(format!("media={}", map_paper_size(&settings.paper_size)));

    // Scale enforcement
    if !settings.fit_to_page {
        let scale_pct = (settings.scale * 100.0).round() as u32;
        cmd.arg("-o").arg(format!("scaling={scale_pct}"));
        cmd.arg("-o").arg("fit-to-page=false");
    } else {
        cmd.arg("-o").arg("fit-to-page=true");
    }

    // Orientation
    if settings.orientation == "landscape" {
        cmd.arg("-o").arg("landscape");
    }

    // Page ranges
    if let Some(ranges) = &settings.page_ranges {
        if !ranges.is_empty() {
            cmd.arg("-o").arg(format!("page-ranges={ranges}"));
        }
    }

    cmd.arg(path);

    let output = cmd.output().map_err(|e| {
        AppError::Internal(format!("lpr konnte nicht ausgefuehrt werden: {e}"))
    })?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(AppError::Internal(format!("Druckfehler: {stderr}")));
    }

    log::info!("Print job submitted for {path}");
    Ok(())
}

#[cfg(target_os = "windows")]
fn print_file_windows(path: &str, settings: &PrintSettings) -> Result<(), AppError> {
    // On Windows, use Start-Process -Verb Print which opens the OS print dialog.
    // printer_name is already validated to be alphanumeric+hyphens+underscores.
    // Start-Process with -Verb Print correctly handles binary PDF data.
    let output = std::process::Command::new("powershell")
        .args([
            "-NoProfile",
            "-Command",
            &format!("Start-Process -FilePath '{}' -Verb Print -Wait", path.replace('\'', "''")),
        ])
        .output()
        .map_err(|e| AppError::Internal(format!("Druckbefehl fehlgeschlagen: {e}")))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(AppError::Internal(format!("Druckfehler: {stderr}")));
    }

    Ok(())
}

#[cfg(target_os = "windows")]
fn list_printers_windows() -> Result<Vec<PrinterInfo>, AppError> {
    let output = std::process::Command::new("powershell")
        .args([
            "-NoProfile", "-Command",
            "Get-Printer | Select-Object -Property Name,Default | ConvertTo-Json"
        ])
        .output()
        .map_err(|e| AppError::Internal(format!("Druckerabfrage fehlgeschlagen: {e}")))?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    // Parse JSON array of { Name: "...", Default: true/false }
    let mut printers = Vec::new();
    if let Ok(arr) = serde_json::from_str::<Vec<serde_json::Value>>(&stdout) {
        for item in arr {
            if let Some(name) = item.get("Name").and_then(|v| v.as_str()) {
                let is_default = item.get("Default").and_then(|v| v.as_bool()).unwrap_or(false);
                printers.push(PrinterInfo {
                    display_name: name.to_string(),
                    name: name.to_string(),
                    is_default,
                });
            }
        }
    }
    Ok(printers)
}

/// Save print settings to the database for persistence.
#[tauri::command]
pub fn save_print_settings(
    db: State<'_, DbState>,
    paper_size: String,
    orientation: String,
    printer_name: Option<String>,
) -> Result<(), AppError> {
    let conn = lock_db(&db)?;
    let pairs = [
        ("print_paper_size", paper_size),
        ("print_orientation", orientation),
        ("print_printer", printer_name.unwrap_or_default()),
    ];
    for (key, value) in &pairs {
        conn.execute(
            "INSERT OR REPLACE INTO settings (key, value, updated_at) VALUES (?1, ?2, datetime('now'))",
            rusqlite::params![key, value],
        )?;
    }
    Ok(())
}

/// Load saved print settings.
#[tauri::command]
pub fn load_print_settings(
    db: State<'_, DbState>,
) -> Result<std::collections::HashMap<String, String>, AppError> {
    let conn = lock_db(&db)?;
    let mut stmt = conn.prepare(
        "SELECT key, value FROM settings WHERE key IN ('print_paper_size', 'print_orientation', 'print_printer')"
    )?;
    let mut map = std::collections::HashMap::new();
    let rows = stmt.query_map([], |row| {
        Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
    })?;
    for row in rows {
        if let Ok((k, v)) = row {
            map.insert(k, v);
        }
    }
    Ok(map)
}

/// Mark a file as recently printed.
#[tauri::command]
pub fn mark_as_printed(
    db: State<'_, DbState>,
    file_id: i64,
) -> Result<(), AppError> {
    let conn = lock_db(&db)?;
    let key = format!("last_printed:{file_id}");
    conn.execute(
        "INSERT OR REPLACE INTO settings (key, value, updated_at) \
         VALUES (?1, datetime('now'), datetime('now'))",
        [&key],
    )?;
    Ok(())
}

/// Get recently printed file IDs (most recent first, up to limit).
#[tauri::command]
pub fn get_recently_printed(
    db: State<'_, DbState>,
    limit: Option<i64>,
) -> Result<Vec<i64>, AppError> {
    let conn = lock_db(&db)?;
    let max = limit.unwrap_or(10);
    let mut stmt = conn.prepare(
        "SELECT CAST(REPLACE(key, 'last_printed:', '') AS INTEGER) AS file_id \
         FROM settings WHERE key LIKE 'last_printed:%' \
         ORDER BY value DESC LIMIT ?1"
    )?;
    let ids = stmt.query_map([max], |row| row.get::<_, i64>(0))?
        .filter_map(|r| r.ok())
        .collect();
    Ok(ids)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_map_paper_size() {
        assert_eq!(map_paper_size("A4"), "A4");
        assert_eq!(map_paper_size("a4"), "A4");
        assert_eq!(map_paper_size("Letter"), "Letter");
        assert_eq!(map_paper_size("US Letter"), "Letter");
        assert_eq!(map_paper_size("A3"), "A3");
        assert_eq!(map_paper_size("unknown"), "A4");
    }

    #[test]
    fn test_printer_info_serialization() {
        let info = PrinterInfo {
            name: "HP_LaserJet".into(),
            display_name: "HP LaserJet".into(),
            is_default: true,
        };
        let json = serde_json::to_string(&info).unwrap();
        assert!(json.contains("\"isDefault\":true"));
        assert!(json.contains("\"displayName\":\"HP LaserJet\""));
    }

    #[test]
    fn test_print_settings_deserialization() {
        let json = r#"{
            "printerName": "MyPrinter",
            "paperSize": "A4",
            "orientation": "portrait",
            "copies": 2,
            "scale": 1.0,
            "fitToPage": false,
            "pageRanges": "1-3,5",
            "tileEnabled": false,
            "tileOverlapMm": 15.0
        }"#;
        let settings: PrintSettings = serde_json::from_str(json).unwrap();
        assert_eq!(settings.printer_name.as_deref(), Some("MyPrinter"));
        assert_eq!(settings.copies, 2);
        assert_eq!(settings.scale, 1.0);
        assert!(!settings.fit_to_page);
        assert_eq!(settings.page_ranges.as_deref(), Some("1-3,5"));
    }
}
