use crate::error::AppError;
use crate::services::thread_db;

#[tauri::command]
pub fn get_thread_matches(
    color_hex: String,
    brands: Option<Vec<String>>,
    limit: Option<usize>,
) -> Result<Vec<thread_db::ThreadMatch>, AppError> {
    let limit = limit.unwrap_or(5);
    let matches = thread_db::find_matches(
        &color_hex,
        brands.as_deref(),
        limit,
    );
    Ok(matches)
}

#[tauri::command]
pub fn get_available_brands() -> Result<Vec<String>, AppError> {
    Ok(thread_db::available_brands())
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BrandColorInfo {
    pub brand: String,
    pub code: String,
    pub name: String,
    pub hex: String,
}

#[tauri::command]
pub fn get_brand_colors(brand: String) -> Result<Vec<BrandColorInfo>, AppError> {
    let colors = thread_db::brand_colors(&brand);
    Ok(colors
        .into_iter()
        .map(|c| BrandColorInfo {
            brand: c.brand.to_string(),
            code: c.code.to_string(),
            name: c.name.to_string(),
            hex: format!("#{:02X}{:02X}{:02X}", c.r, c.g, c.b),
        })
        .collect())
}
