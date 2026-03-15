use crate::error::AppError;
use crate::parsers::{EmbroideryParser, ParsedFileInfo, StitchSegment};

pub struct PdfParser;

impl EmbroideryParser for PdfParser {
    fn supported_extensions(&self) -> &[&str] {
        &["pdf"]
    }

    fn parse(&self, data: &[u8]) -> Result<ParsedFileInfo, AppError> {
        let doc = lopdf::Document::load_mem(data)
            .map_err(|e| AppError::Parse { format: "PDF".into(), message: format!("{e}") })?;

        let page_count = doc.get_pages().len() as i32;

        // Extract paper size from first page MediaBox
        let paper_size = extract_paper_size(&doc);

        // Extract title from document info dictionary
        let title = extract_info_string(&doc, b"Title");
        let author = extract_info_string(&doc, b"Author");
        let keywords = extract_info_string(&doc, b"Keywords");

        // Extract page dimensions in mm (from first page MediaBox, 1 pt = 0.3528 mm)
        let (width_mm, height_mm) = extract_page_dimensions_mm(&doc);

        Ok(ParsedFileInfo {
            format: "PDF".to_string(),
            format_version: doc.version.clone().into(),
            width_mm,
            height_mm,
            stitch_count: None,
            color_count: None,
            colors: vec![],
            design_name: title,
            jump_count: None,
            trim_count: None,
            hoop_width_mm: None,
            hoop_height_mm: None,
            category: None,
            author,
            keywords,
            comments: None,
            page_count: Some(page_count),
            paper_size,
        })
    }

    fn extract_thumbnail(&self, _data: &[u8]) -> Result<Option<Vec<u8>>, AppError> {
        // PDF thumbnail generation requires a rendering engine (pdfium/pdf.js).
        // Deferred to Sprint 3 when the document viewer is implemented.
        Ok(None)
    }

    fn extract_stitch_segments(&self, _data: &[u8]) -> Result<Vec<StitchSegment>, AppError> {
        Ok(vec![])
    }
}

/// Extract paper size label from first page dimensions.
fn extract_paper_size(doc: &lopdf::Document) -> Option<String> {
    let pages = doc.get_pages();
    let first_page_id = pages.values().next()?;
    let page = doc.get_dictionary(*first_page_id).ok()?;

    let media_box = page
        .get(b"MediaBox")
        .ok()
        .and_then(|v| doc.dereference(v).ok())
        .and_then(|(_, v)| v.as_array().ok())?;

    if media_box.len() < 4 {
        return None;
    }

    let llx = get_float(&media_box[0]).unwrap_or(0.0);
    let lly = get_float(&media_box[1]).unwrap_or(0.0);
    let urx = get_float(&media_box[2])?;
    let ury = get_float(&media_box[3])?;
    let width_pt = urx - llx;
    let height_pt = ury - lly;

    classify_paper_size(width_pt, height_pt)
}

/// Extract page dimensions in mm from first page MediaBox.
fn extract_page_dimensions_mm(doc: &lopdf::Document) -> (Option<f64>, Option<f64>) {
    let pages = doc.get_pages();
    let first_page_id = match pages.values().next() {
        Some(id) => id,
        None => return (None, None),
    };
    let page = match doc.get_dictionary(*first_page_id) {
        Ok(p) => p,
        Err(_) => return (None, None),
    };

    let media_box = page
        .get(b"MediaBox")
        .ok()
        .and_then(|v| doc.dereference(v).ok())
        .and_then(|(_, v)| v.as_array().ok());

    match media_box {
        Some(mb) if mb.len() >= 4 => {
            let llx = get_float(&mb[0]).unwrap_or(0.0);
            let lly = get_float(&mb[1]).unwrap_or(0.0);
            let w = get_float(&mb[2]).map(|urx| (urx - llx) * 0.3528);
            let h = get_float(&mb[3]).map(|ury| (ury - lly) * 0.3528);
            (w, h)
        }
        _ => (None, None),
    }
}

/// Classify paper size from point dimensions (tolerance < 3pt).
fn classify_paper_size(width_pt: f64, height_pt: f64) -> Option<String> {
    // Normalize to portrait orientation for comparison
    let (w, h) = if width_pt > height_pt {
        (height_pt, width_pt)
    } else {
        (width_pt, height_pt)
    };

    let sizes = [
        ("A4", 595.0, 842.0),
        ("A3", 842.0, 1191.0),
        ("A2", 1191.0, 1684.0),
        ("A1", 1684.0, 2384.0),
        ("A0", 2384.0, 3370.0),
        ("US Letter", 612.0, 792.0),
        ("US Legal", 612.0, 1008.0),
        ("US Tabloid", 792.0, 1224.0),
    ];

    for (name, sw, sh) in &sizes {
        if (w - sw).abs() < 3.0 && (h - sh).abs() < 3.0 {
            return Some(name.to_string());
        }
    }

    Some(format!("{:.0}x{:.0}pt", width_pt, height_pt))
}

/// Extract a string value from the PDF info dictionary.
fn extract_info_string(doc: &lopdf::Document, key: &[u8]) -> Option<String> {
    let trailer = &doc.trailer;
    let info_ref = trailer.get(b"Info").ok()?;
    let info = match info_ref {
        lopdf::Object::Reference(r) => doc.get_dictionary(*r).ok()?,
        lopdf::Object::Dictionary(d) => d,
        _ => return None,
    };
    let value = info.get(key).ok()?;
    match value {
        lopdf::Object::String(bytes, _) => {
            let s = String::from_utf8_lossy(bytes).trim().to_string();
            if s.is_empty() { None } else { Some(s) }
        }
        _ => None,
    }
}

/// Get a float from a PDF object (Integer or Real).
fn get_float(obj: &lopdf::Object) -> Option<f64> {
    match obj {
        lopdf::Object::Integer(i) => Some(*i as f64),
        lopdf::Object::Real(f) => Some(*f as f64),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_classify_paper_a4() {
        assert_eq!(classify_paper_size(595.0, 842.0), Some("A4".to_string()));
    }

    #[test]
    fn test_classify_paper_a4_landscape() {
        assert_eq!(classify_paper_size(842.0, 595.0), Some("A4".to_string()));
    }

    #[test]
    fn test_classify_paper_us_letter() {
        assert_eq!(classify_paper_size(612.0, 792.0), Some("US Letter".to_string()));
    }

    #[test]
    fn test_classify_paper_custom() {
        let result = classify_paper_size(500.0, 700.0);
        assert_eq!(result, Some("500x700pt".to_string()));
    }

    #[test]
    fn test_pdf_parser_extensions() {
        let parser = PdfParser;
        assert_eq!(parser.supported_extensions(), &["pdf"]);
    }

    #[test]
    fn test_pdf_no_stitch_segments() {
        let parser = PdfParser;
        let segments = parser.extract_stitch_segments(&[]).unwrap();
        assert!(segments.is_empty());
    }

    #[test]
    fn test_pdf_no_thumbnail() {
        let parser = PdfParser;
        let thumb = parser.extract_thumbnail(&[]).unwrap();
        assert!(thumb.is_none());
    }
}
