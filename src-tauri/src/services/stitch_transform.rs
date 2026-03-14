use crate::parsers::StitchSegment;

/// Resize all stitch coordinates by scale factors.
/// Returns an error string if scale factors are invalid.
pub fn resize(segments: &mut [StitchSegment], scale_x: f64, scale_y: f64) {
    // Guard against invalid scale factors
    if !scale_x.is_finite() || !scale_y.is_finite() || scale_x == 0.0 || scale_y == 0.0 {
        return;
    }
    for seg in segments.iter_mut() {
        for pt in seg.points.iter_mut() {
            pt.0 *= scale_x;
            pt.1 *= scale_y;
        }
    }
}

/// Rotate all stitch coordinates around the center by the given angle in degrees.
pub fn rotate(segments: &mut [StitchSegment], degrees: f64) {
    let (cx, cy) = center(segments);
    let rad = degrees.to_radians();
    let cos = rad.cos();
    let sin = rad.sin();

    for seg in segments.iter_mut() {
        for pt in seg.points.iter_mut() {
            let dx = pt.0 - cx;
            let dy = pt.1 - cy;
            pt.0 = cx + dx * cos - dy * sin;
            pt.1 = cy + dx * sin + dy * cos;
        }
    }
}

/// Mirror horizontally (flip left-right) around the center X axis.
pub fn mirror_horizontal(segments: &mut [StitchSegment]) {
    let (cx, _) = center(segments);
    for seg in segments.iter_mut() {
        for pt in seg.points.iter_mut() {
            pt.0 = 2.0 * cx - pt.0;
        }
    }
}

/// Mirror vertically (flip top-bottom) around the center Y axis.
pub fn mirror_vertical(segments: &mut [StitchSegment]) {
    let (_, cy) = center(segments);
    for seg in segments.iter_mut() {
        for pt in seg.points.iter_mut() {
            pt.1 = 2.0 * cy - pt.1;
        }
    }
}

/// Calculate the bounding box of all points.
/// Returns None if no points exist.
fn bounding_box(segments: &[StitchSegment]) -> Option<(f64, f64, f64, f64)> {
    let mut min_x = f64::MAX;
    let mut max_x = f64::MIN;
    let mut min_y = f64::MAX;
    let mut max_y = f64::MIN;
    let mut has_points = false;

    for seg in segments {
        for &(x, y) in &seg.points {
            has_points = true;
            min_x = min_x.min(x);
            max_x = max_x.max(x);
            min_y = min_y.min(y);
            max_y = max_y.max(y);
        }
    }

    if has_points { Some((min_x, min_y, max_x, max_y)) } else { None }
}

/// Calculate the bounding box center of all points.
/// Returns (0, 0) if no points exist.
fn center(segments: &[StitchSegment]) -> (f64, f64) {
    match bounding_box(segments) {
        Some((min_x, min_y, max_x, max_y)) => ((min_x + max_x) / 2.0, (min_y + max_y) / 2.0),
        None => (0.0, 0.0),
    }
}

/// Calculate bounding box dimensions in mm.
pub fn dimensions(segments: &[StitchSegment]) -> (f64, f64) {
    match bounding_box(segments) {
        Some((min_x, min_y, max_x, max_y)) => ((max_x - min_x).max(0.0), (max_y - min_y).max(0.0)),
        None => (0.0, 0.0),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_segments() -> Vec<StitchSegment> {
        vec![StitchSegment {
            color_index: 0,
            color_hex: Some("#FF0000".to_string()),
            points: vec![(0.0, 0.0), (10.0, 0.0), (10.0, 10.0), (0.0, 10.0)],
        }]
    }

    #[test]
    fn test_resize() {
        let mut segs = test_segments();
        resize(&mut segs, 2.0, 3.0);
        assert_eq!(segs[0].points[1], (20.0, 0.0));
        assert_eq!(segs[0].points[2], (20.0, 30.0));
    }

    #[test]
    fn test_rotate_90() {
        let mut segs = test_segments();
        rotate(&mut segs, 90.0);
        // After 90° rotation around center (5,5):
        // (10,0) -> (5 + (0-5), 5 + (10-5)*1) -> (10, 10) approx
        let (w, h) = dimensions(&segs);
        assert!((w - 10.0).abs() < 0.01);
        assert!((h - 10.0).abs() < 0.01);
    }

    #[test]
    fn test_mirror_horizontal() {
        let mut segs = test_segments();
        mirror_horizontal(&mut segs);
        // Center X = 5.0; point (0,0) -> (10, 0)
        assert!((segs[0].points[0].0 - 10.0).abs() < 0.001);
        assert!((segs[0].points[1].0 - 0.0).abs() < 0.001);
    }

    #[test]
    fn test_mirror_vertical() {
        let mut segs = test_segments();
        mirror_vertical(&mut segs);
        // Center Y = 5.0; point (0,0) -> (0, 10)
        assert!((segs[0].points[0].1 - 10.0).abs() < 0.001);
        assert!((segs[0].points[3].1 - 0.0).abs() < 0.001);
    }

    #[test]
    fn test_dimensions() {
        let segs = test_segments();
        let (w, h) = dimensions(&segs);
        assert_eq!(w, 10.0);
        assert_eq!(h, 10.0);
    }
}
