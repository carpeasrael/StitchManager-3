pub mod dst;
pub mod pes;

use serde::Serialize;

use crate::error::AppError;

/// Parsed metadata from an embroidery file.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ParsedFileInfo {
    pub format: String,
    pub format_version: Option<String>,
    pub width_mm: Option<f64>,
    pub height_mm: Option<f64>,
    pub stitch_count: Option<u32>,
    pub color_count: Option<u16>,
    pub colors: Vec<ParsedColor>,
}

/// A single thread color extracted from the file.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ParsedColor {
    pub hex: String,
    pub name: Option<String>,
    pub brand: Option<String>,
    pub brand_code: Option<String>,
}

/// Common interface for all embroidery format parsers.
#[allow(dead_code)]
pub trait EmbroideryParser: Send + Sync {
    fn supported_extensions(&self) -> &[&str];
    fn parse(&self, data: &[u8]) -> Result<ParsedFileInfo, AppError>;
    fn extract_thumbnail(&self, data: &[u8]) -> Result<Option<Vec<u8>>, AppError>;
}

/// Look up a parser implementation by file extension (case-insensitive).
pub fn get_parser(extension: &str) -> Option<&'static dyn EmbroideryParser> {
    static PES_PARSER: pes::PesParser = pes::PesParser;
    static DST_PARSER: dst::DstParser = dst::DstParser;

    match extension.to_lowercase().as_str() {
        "pes" => Some(&PES_PARSER),
        "dst" => Some(&DST_PARSER),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_parser_pes() {
        let parser = get_parser("pes").unwrap();
        assert!(parser.supported_extensions().contains(&"pes"));
    }

    #[test]
    fn test_get_parser_dst() {
        let parser = get_parser("dst").unwrap();
        assert!(parser.supported_extensions().contains(&"dst"));
    }

    #[test]
    fn test_get_parser_case_insensitive() {
        assert!(get_parser("PES").is_some());
        assert!(get_parser("Dst").is_some());
    }

    #[test]
    fn test_get_parser_unknown() {
        assert!(get_parser("png").is_none());
        assert!(get_parser("jef").is_none()); // Not yet implemented
    }
}
