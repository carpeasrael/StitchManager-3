use serde::{Deserialize, Serialize};

// Note on bool fields: SQLite stores booleans as INTEGER (0/1). rusqlite handles
// the conversion automatically. When the frontend queries via tauri-plugin-sql,
// it receives raw 0/1 integers — the TypeScript layer must coerce these to boolean.
// Rust-side Tauri commands serialize these as JSON booleans via serde.

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SchemaVersion {
    pub version: i32,
    pub applied_at: String,
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Folder {
    pub id: i64,
    pub name: String,
    pub path: String,
    pub parent_id: Option<i64>,
    pub sort_order: i32,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EmbroideryFile {
    pub id: i64,
    pub folder_id: i64,
    pub filename: String,
    pub filepath: String,
    pub name: Option<String>,
    pub theme: Option<String>,
    pub description: Option<String>,
    pub license: Option<String>,
    pub width_mm: Option<f64>,
    pub height_mm: Option<f64>,
    pub stitch_count: Option<i32>,
    pub color_count: Option<i32>,
    pub file_size_bytes: Option<i64>,
    pub thumbnail_path: Option<String>,
    pub design_name: Option<String>,
    pub jump_count: Option<i32>,
    pub trim_count: Option<i32>,
    pub hoop_width_mm: Option<f64>,
    pub hoop_height_mm: Option<f64>,
    pub category: Option<String>,
    pub author: Option<String>,
    pub keywords: Option<String>,
    pub comments: Option<String>,
    pub unique_id: Option<String>,
    pub ai_analyzed: bool,
    pub ai_confirmed: bool,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FileFormat {
    pub id: i64,
    pub file_id: i64,
    pub format: String,
    pub format_version: Option<String>,
    pub filepath: String,
    pub file_size_bytes: Option<i64>,
    pub parsed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FileThreadColor {
    pub id: i64,
    pub file_id: i64,
    pub sort_order: i32,
    pub color_hex: String,
    pub color_name: Option<String>,
    pub brand: Option<String>,
    pub brand_code: Option<String>,
    pub is_ai: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Tag {
    pub id: i64,
    pub name: String,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FileTag {
    pub file_id: i64,
    pub tag_id: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AiAnalysisResult {
    pub id: i64,
    pub file_id: i64,
    pub provider: String,
    pub model: String,
    pub prompt_hash: Option<String>,
    pub raw_response: Option<String>,
    pub parsed_name: Option<String>,
    pub parsed_theme: Option<String>,
    pub parsed_desc: Option<String>,
    pub parsed_tags: Option<String>,
    pub parsed_colors: Option<String>,
    pub accepted: bool,
    pub analyzed_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Setting {
    pub key: String,
    pub value: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CustomFieldDefinition {
    pub id: i64,
    pub name: String,
    pub field_type: String,
    pub options: Option<String>,
    pub required: bool,
    pub sort_order: i32,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CustomFieldValue {
    pub file_id: i64,
    pub field_id: i64,
    pub value: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FileAttachment {
    pub id: i64,
    pub file_id: i64,
    pub filename: String,
    pub mime_type: Option<String>,
    pub file_path: String,
    pub attachment_type: String,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FileUpdate {
    pub name: Option<String>,
    pub theme: Option<String>,
    pub description: Option<String>,
    pub license: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PaginatedFiles {
    pub files: Vec<EmbroideryFile>,
    pub total_count: i64,
    pub page: i64,
    pub page_size: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct SearchParams {
    /// Free-text query: searches name, filename, theme, description,
    /// design_name, category, author, keywords, comments, license
    pub text: Option<String>,

    /// Tags: file must have ALL listed tags (AND logic)
    pub tags: Option<Vec<String>>,

    /// Numeric range filters
    pub stitch_count_min: Option<i32>,
    pub stitch_count_max: Option<i32>,
    pub color_count_min: Option<i32>,
    pub color_count_max: Option<i32>,
    pub width_mm_min: Option<f64>,
    pub width_mm_max: Option<f64>,
    pub height_mm_min: Option<f64>,
    pub height_mm_max: Option<f64>,
    pub file_size_min: Option<i64>,
    pub file_size_max: Option<i64>,

    /// Boolean filters
    pub ai_analyzed: Option<bool>,
    pub ai_confirmed: Option<bool>,

    /// Thread color name or brand search
    pub color_search: Option<String>,
}
