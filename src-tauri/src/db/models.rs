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
    pub is_favorite: bool,
    pub file_type: String,
    pub size_range: Option<String>,
    pub skill_level: Option<String>,
    pub language: Option<String>,
    pub format_type: Option<String>,
    pub file_source: Option<String>,
    pub purchase_link: Option<String>,
    pub status: String,
    pub page_count: Option<i32>,
    pub paper_size: Option<String>,
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
    pub display_name: Option<String>,
    pub sort_order: i32,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FileUpdate {
    pub name: Option<String>,
    pub theme: Option<String>,
    pub description: Option<String>,
    pub license: Option<String>,
    pub size_range: Option<String>,
    pub skill_level: Option<String>,
    pub language: Option<String>,
    pub format_type: Option<String>,
    pub file_source: Option<String>,
    pub purchase_link: Option<String>,
    pub status: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PaginatedFiles {
    pub files: Vec<EmbroideryFile>,
    pub total_count: i64,
    pub page: i64,
    pub page_size: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InstructionBookmark {
    pub id: i64,
    pub file_id: i64,
    pub page_number: i32,
    pub label: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InstructionNote {
    pub id: i64,
    pub file_id: i64,
    pub page_number: i32,
    pub note_text: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Project {
    pub id: i64,
    pub name: String,
    pub pattern_file_id: Option<i64>,
    pub status: String,
    pub notes: Option<String>,
    pub order_number: Option<String>,
    pub customer: Option<String>,
    pub priority: Option<String>,
    pub deadline: Option<String>,
    pub responsible_person: Option<String>,
    pub approval_status: Option<String>,
    pub quantity: Option<i64>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectDetail {
    pub id: i64,
    pub project_id: i64,
    pub key: String,
    pub value: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Collection {
    pub id: i64,
    pub name: String,
    pub description: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Supplier {
    pub id: i64,
    pub name: String,
    pub contact: Option<String>,
    pub website: Option<String>,
    pub notes: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Material {
    pub id: i64,
    pub material_number: Option<String>,
    pub name: String,
    pub material_type: Option<String>,
    pub unit: Option<String>,
    pub supplier_id: Option<i64>,
    pub net_price: Option<f64>,
    pub waste_factor: Option<f64>,
    pub min_stock: Option<f64>,
    pub reorder_time_days: Option<i32>,
    pub notes: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MaterialInventory {
    pub id: i64,
    pub material_id: i64,
    pub total_stock: f64,
    pub reserved_stock: f64,
    pub location: Option<String>,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MaterialConsumption {
    pub id: i64,
    pub project_id: i64,
    pub material_id: i64,
    pub quantity: f64,
    pub unit: Option<String>,
    pub step_name: Option<String>,
    pub recorded_by: Option<String>,
    pub notes: Option<String>,
    pub recorded_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InventoryTransaction {
    pub id: i64,
    pub material_id: i64,
    pub project_id: Option<i64>,
    pub transaction_type: String,
    pub quantity: f64,
    pub notes: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NachkalkulationLine {
    pub material_id: i64,
    pub material_name: String,
    pub unit: Option<String>,
    pub planned_quantity: f64,
    pub actual_quantity: f64,
    pub difference: f64,
    pub planned_cost: f64,
    pub actual_cost: f64,
    pub cost_difference: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Product {
    pub id: i64,
    pub product_number: Option<String>,
    pub name: String,
    pub category: Option<String>,
    pub description: Option<String>,
    pub product_type: Option<String>,
    pub status: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProductVariant {
    pub id: i64,
    pub product_id: i64,
    pub sku: Option<String>,
    pub variant_name: Option<String>,
    pub size: Option<String>,
    pub color: Option<String>,
    pub additional_cost: f64,
    pub notes: Option<String>,
    pub status: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BillOfMaterial {
    pub id: i64,
    pub product_id: i64,
    pub material_id: i64,
    pub quantity: f64,
    pub unit: Option<String>,
    pub notes: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TimeEntry {
    pub id: i64,
    pub project_id: i64,
    pub step_name: String,
    pub planned_minutes: Option<f64>,
    pub actual_minutes: Option<f64>,
    pub worker: Option<String>,
    pub machine: Option<String>,
    pub cost_rate_id: Option<i64>,
    pub recorded_at: String,
}

// Sprint D: Production Workflow

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StepDefinition {
    pub id: i64,
    pub name: String,
    pub description: Option<String>,
    pub default_duration_minutes: Option<f64>,
    pub sort_order: i32,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProductStep {
    pub id: i64,
    pub product_id: i64,
    pub step_definition_id: i64,
    pub sort_order: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkflowStep {
    pub id: i64,
    pub project_id: i64,
    pub step_definition_id: i64,
    pub status: String,
    pub responsible: Option<String>,
    pub started_at: Option<String>,
    pub completed_at: Option<String>,
    pub notes: Option<String>,
    pub sort_order: i32,
}

// Sprint E: Procurement

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PurchaseOrder {
    pub id: i64,
    pub order_number: Option<String>,
    pub supplier_id: i64,
    pub project_id: Option<i64>,
    pub status: String,
    pub order_date: Option<String>,
    pub expected_delivery: Option<String>,
    pub shipping_cost: f64,
    pub notes: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OrderItem {
    pub id: i64,
    pub order_id: i64,
    pub material_id: i64,
    pub quantity_ordered: f64,
    pub quantity_delivered: f64,
    pub unit_price: Option<f64>,
    pub notes: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Delivery {
    pub id: i64,
    pub order_id: i64,
    pub delivery_date: String,
    pub delivery_note: Option<String>,
    pub notes: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MaterialRequirement {
    pub material_id: i64,
    pub material_name: String,
    pub unit: Option<String>,
    pub needed: f64,
    pub available: f64,
    pub shortage: f64,
    pub supplier_id: Option<i64>,
    pub supplier_name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeliveryItem {
    pub id: i64,
    pub delivery_id: i64,
    pub order_item_id: i64,
    pub quantity_received: f64,
}

// Sprint F: License Management

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LicenseRecord {
    pub id: i64,
    pub name: String,
    pub license_type: Option<String>,
    pub valid_from: Option<String>,
    pub valid_until: Option<String>,
    pub max_uses: Option<i32>,
    pub current_uses: i32,
    pub commercial_allowed: bool,
    pub cost_per_piece: f64,
    pub cost_per_series: f64,
    pub cost_flat: f64,
    pub source: Option<String>,
    pub notes: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

// Phase 3: Quality Management

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QualityInspection {
    pub id: i64,
    pub project_id: i64,
    pub workflow_step_id: Option<i64>,
    pub inspector: Option<String>,
    pub inspection_date: String,
    pub result: String,
    pub notes: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DefectRecord {
    pub id: i64,
    pub inspection_id: i64,
    pub description: String,
    pub severity: Option<String>,
    pub status: Option<String>,
    pub resolved_at: Option<String>,
    pub notes: Option<String>,
    pub created_at: String,
}

// Cost Calculation

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CostRate {
    pub id: i64,
    pub rate_type: String,
    pub name: String,
    pub rate_value: f64,
    pub unit: Option<String>,
    pub setup_cost: f64,
    pub notes: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CostBreakdown {
    pub project_id: i64,
    pub project_name: String,
    pub quantity: i64,
    pub material_cost: f64,
    pub license_cost: f64,
    pub stitch_cost: f64,
    pub labor_cost: f64,
    pub machine_cost: f64,
    pub procurement_cost: f64,
    pub herstellkosten: f64,
    pub overhead_pct: f64,
    pub overhead_cost: f64,
    pub selbstkosten: f64,
    pub profit_margin_pct: f64,
    pub profit_amount: f64,
    pub netto_verkaufspreis: f64,
    pub selbstkosten_per_piece: f64,
    pub verkaufspreis_per_piece: f64,
}

// Phase 3: Reporting

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectReport {
    pub project_id: i64,
    pub project_name: String,
    pub total_planned_minutes: f64,
    pub total_actual_minutes: f64,
    pub material_cost: f64,
    pub labor_cost: f64,
    pub total_cost: f64,
    pub inspection_count: i64,
    pub pass_count: i64,
    pub fail_count: i64,
    pub open_defects: i64,
    pub workflow_total: i64,
    pub workflow_completed: i64,
    pub cost_breakdown: Option<CostBreakdown>,
}

// Audit Trail

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AuditLogEntry {
    pub id: i64,
    pub entity_type: String,
    pub entity_id: i64,
    pub field_name: String,
    pub old_value: Option<String>,
    pub new_value: Option<String>,
    pub changed_by: Option<String>,
    pub changed_at: String,
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

    /// File type discriminator filter
    pub file_type: Option<String>,
    /// Status filter
    pub status: Option<String>,
    /// Skill level filter
    pub skill_level: Option<String>,
    /// Language filter
    pub language: Option<String>,
    /// File source filter
    pub file_source: Option<String>,
    /// Category / garment type filter
    pub category: Option<String>,
    /// Author / designer filter
    pub author: Option<String>,
    /// Size range text match filter
    pub size_range: Option<String>,
    /// Sort field: filename, name, created_at, updated_at, author, category
    pub sort_field: Option<String>,
    /// Sort direction: asc or desc (default: asc)
    pub sort_direction: Option<String>,
}
