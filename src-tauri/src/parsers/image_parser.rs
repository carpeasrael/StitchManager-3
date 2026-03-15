use crate::error::AppError;
use crate::parsers::{EmbroideryParser, ParsedFileInfo, StitchSegment};

/// Default DPI assumption for converting pixel dimensions to mm.
const DEFAULT_DPI: f64 = 96.0;
const MM_PER_INCH: f64 = 25.4;

pub struct ImageParser;

impl EmbroideryParser for ImageParser {
    fn supported_extensions(&self) -> &[&str] {
        &["png", "jpg", "jpeg", "bmp"]
    }

    fn parse(&self, data: &[u8]) -> Result<ParsedFileInfo, AppError> {
        let img = image::load_from_memory(data)
            .map_err(|e| AppError::Parse { format: "IMAGE".into(), message: format!("{e}") })?;

        let (w_px, h_px) = (img.width() as f64, img.height() as f64);
        let width_mm = w_px / DEFAULT_DPI * MM_PER_INCH;
        let height_mm = h_px / DEFAULT_DPI * MM_PER_INCH;

        // Detect format from data header
        let format = image::guess_format(data)
            .map(|f| format!("{f:?}").to_uppercase())
            .unwrap_or_else(|_| "IMAGE".to_string());

        Ok(ParsedFileInfo {
            format,
            format_version: None,
            width_mm: Some(width_mm),
            height_mm: Some(height_mm),
            stitch_count: None,
            color_count: None,
            colors: vec![],
            design_name: None,
            jump_count: None,
            trim_count: None,
            hoop_width_mm: None,
            hoop_height_mm: None,
            category: None,
            author: None,
            keywords: None,
            comments: None,
            page_count: None,
            paper_size: None,
        })
    }

    fn extract_thumbnail(&self, data: &[u8]) -> Result<Option<Vec<u8>>, AppError> {
        let img = image::load_from_memory(data)
            .map_err(|e| AppError::Parse { format: "IMAGE".into(), message: format!("{e}") })?;

        let thumb = img.thumbnail(192, 192);
        let mut buf = std::io::Cursor::new(Vec::new());
        thumb
            .write_to(&mut buf, image::ImageFormat::Png)
            .map_err(|e| AppError::Internal(format!("Thumbnail encode error: {e}")))?;

        Ok(Some(buf.into_inner()))
    }

    fn extract_stitch_segments(&self, _data: &[u8]) -> Result<Vec<StitchSegment>, AppError> {
        Ok(vec![])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_image_parser_extensions() {
        let parser = ImageParser;
        let exts = parser.supported_extensions();
        assert!(exts.contains(&"png"));
        assert!(exts.contains(&"jpg"));
        assert!(exts.contains(&"jpeg"));
        assert!(exts.contains(&"bmp"));
    }

    #[test]
    fn test_image_no_stitch_segments() {
        let parser = ImageParser;
        let segments = parser.extract_stitch_segments(&[]).unwrap();
        assert!(segments.is_empty());
    }

    #[test]
    fn test_parse_generated_png() {
        // Generate a valid 2x2 PNG via the image crate
        let img = image::RgbImage::from_pixel(2, 2, image::Rgb([255, 0, 0]));
        let mut buf = std::io::Cursor::new(Vec::new());
        img.write_to(&mut buf, image::ImageFormat::Png).unwrap();
        let png_data = buf.into_inner();

        let parser = ImageParser;
        let info = parser.parse(&png_data).unwrap();
        assert!(info.width_mm.is_some());
        assert!(info.height_mm.is_some());
        assert!(info.page_count.is_none());

        // Thumbnail should also work
        let thumb = parser.extract_thumbnail(&png_data).unwrap();
        assert!(thumb.is_some());
    }
}
