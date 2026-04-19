pub mod dst;
pub mod image_parser;
pub mod jef;
pub mod pdf;
pub mod pes;
pub mod vp3;
pub mod writers;

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
    pub stitch_count: Option<i32>,
    pub color_count: Option<i32>,
    pub colors: Vec<ParsedColor>,
    pub design_name: Option<String>,
    pub jump_count: Option<i32>,
    pub trim_count: Option<i32>,
    pub hoop_width_mm: Option<f64>,
    pub hoop_height_mm: Option<f64>,
    pub category: Option<String>,
    pub author: Option<String>,
    pub keywords: Option<String>,
    pub comments: Option<String>,
    pub page_count: Option<i32>,
    pub paper_size: Option<String>,
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

/// A segment of stitches for a single color layer.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StitchSegment {
    pub color_index: usize,
    pub color_hex: Option<String>,
    pub points: Vec<(f64, f64)>,
}

/// Common interface for all embroidery format parsers.
#[allow(dead_code)] // supported_extensions() is used in tests only
pub trait EmbroideryParser: Send + Sync {
    fn supported_extensions(&self) -> &[&str];
    fn parse(&self, data: &[u8]) -> Result<ParsedFileInfo, AppError>;
    fn extract_thumbnail(&self, data: &[u8]) -> Result<Option<Vec<u8>>, AppError>;
    fn extract_stitch_segments(&self, data: &[u8]) -> Result<Vec<StitchSegment>, AppError>;
}

/// Look up a parser implementation by file extension (case-insensitive).
pub fn get_parser(extension: &str) -> Option<&'static dyn EmbroideryParser> {
    static PES_PARSER: pes::PesParser = pes::PesParser;
    static DST_PARSER: dst::DstParser = dst::DstParser;
    static JEF_PARSER: jef::JefParser = jef::JefParser;
    static VP3_PARSER: vp3::Vp3Parser = vp3::Vp3Parser;
    static PDF_PARSER: pdf::PdfParser = pdf::PdfParser;
    static IMAGE_PARSER: image_parser::ImageParser = image_parser::ImageParser;

    // Audit Wave 5 (deferred from Wave 2 perf #22): match without an
    // intermediate `to_lowercase()` allocation — `eq_ignore_ascii_case`
    // is allocation-free and equally robust for these short ASCII tokens.
    let ext = extension;
    if ext.eq_ignore_ascii_case("pes") { return Some(&PES_PARSER); }
    if ext.eq_ignore_ascii_case("dst") { return Some(&DST_PARSER); }
    if ext.eq_ignore_ascii_case("jef") { return Some(&JEF_PARSER); }
    if ext.eq_ignore_ascii_case("vp3") { return Some(&VP3_PARSER); }
    if ext.eq_ignore_ascii_case("pdf") { return Some(&PDF_PARSER); }
    if ext.eq_ignore_ascii_case("png")
        || ext.eq_ignore_ascii_case("jpg")
        || ext.eq_ignore_ascii_case("jpeg")
        || ext.eq_ignore_ascii_case("bmp")
    {
        return Some(&IMAGE_PARSER);
    }
    None
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
    fn test_get_parser_jef() {
        let parser = get_parser("jef").unwrap();
        assert!(parser.supported_extensions().contains(&"jef"));
    }

    #[test]
    fn test_get_parser_vp3() {
        let parser = get_parser("vp3").unwrap();
        assert!(parser.supported_extensions().contains(&"vp3"));
    }

    #[test]
    fn test_get_parser_pdf() {
        let parser = get_parser("pdf").unwrap();
        assert!(parser.supported_extensions().contains(&"pdf"));
    }

    #[test]
    fn test_get_parser_image() {
        assert!(get_parser("png").is_some());
        assert!(get_parser("jpg").is_some());
        assert!(get_parser("jpeg").is_some());
        assert!(get_parser("bmp").is_some());
    }

    #[test]
    fn test_get_parser_unknown() {
        assert!(get_parser("xxx").is_none());
        assert!(get_parser("svg").is_none());
    }
}
