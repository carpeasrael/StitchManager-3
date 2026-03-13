use std::sync::OnceLock;

/// A single thread color entry from a manufacturer's catalog.
#[derive(Debug, Clone)]
pub struct ThreadBrandColor {
    pub brand: &'static str,
    pub code: &'static str,
    pub name: &'static str,
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

/// A match result with perceptual distance.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ThreadMatch {
    pub brand: String,
    pub code: String,
    pub name: String,
    pub hex: String,
    pub delta_e: f64,
}

/// Pre-computed Lab values for fast matching.
struct LabEntry {
    color: &'static ThreadBrandColor,
    l: f64,
    a: f64,
    b: f64,
}

static LAB_CACHE: OnceLock<Vec<LabEntry>> = OnceLock::new();

fn get_lab_cache() -> &'static Vec<LabEntry> {
    LAB_CACHE.get_or_init(|| {
        all_thread_colors()
            .iter()
            .map(|c| {
                let (l, a, b) = rgb_to_lab(c.r, c.g, c.b);
                LabEntry { color: c, l, a, b }
            })
            .collect()
    })
}

/// Convert sRGB to CIE Lab (D65 illuminant).
fn rgb_to_lab(r: u8, g: u8, b: u8) -> (f64, f64, f64) {
    // sRGB to linear
    let linearize = |c: u8| -> f64 {
        let v = c as f64 / 255.0;
        if v <= 0.04045 {
            v / 12.92
        } else {
            ((v + 0.055) / 1.055).powf(2.4)
        }
    };

    let rl = linearize(r);
    let gl = linearize(g);
    let bl = linearize(b);

    // Linear RGB to XYZ (D65)
    let x = 0.4124564 * rl + 0.3575761 * gl + 0.1804375 * bl;
    let y = 0.2126729 * rl + 0.7151522 * gl + 0.0721750 * bl;
    let z = 0.0193339 * rl + 0.1191920 * gl + 0.9503041 * bl;

    // D65 reference white
    let xn = 0.95047;
    let yn = 1.00000;
    let zn = 1.08883;

    let f = |t: f64| -> f64 {
        if t > 0.008856 {
            t.cbrt()
        } else {
            7.787 * t + 16.0 / 116.0
        }
    };

    let fx = f(x / xn);
    let fy = f(y / yn);
    let fz = f(z / zn);

    let l = 116.0 * fy - 16.0;
    let a = 500.0 * (fx - fy);
    let b_val = 200.0 * (fy - fz);

    (l, a, b_val)
}

/// CIE Delta E 2000 color difference.
fn ciede2000(l1: f64, a1: f64, b1: f64, l2: f64, a2: f64, b2: f64) -> f64 {
    use std::f64::consts::PI;

    // 25^7 — scaling constant used in the CIEDE2000 chroma weighting
    const POW25_7: f64 = 6103515625.0;

    let c1 = (a1 * a1 + b1 * b1).sqrt();
    let c2 = (a2 * a2 + b2 * b2).sqrt();
    let c_avg = (c1 + c2) / 2.0;

    let c_avg_pow7 = c_avg.powi(7);
    let g = 0.5 * (1.0 - (c_avg_pow7 / (c_avg_pow7 + POW25_7)).sqrt());

    let a1p = a1 * (1.0 + g);
    let a2p = a2 * (1.0 + g);

    let c1p = (a1p * a1p + b1 * b1).sqrt();
    let c2p = (a2p * a2p + b2 * b2).sqrt();

    let h1p = b1.atan2(a1p).to_degrees().rem_euclid(360.0);
    let h2p = b2.atan2(a2p).to_degrees().rem_euclid(360.0);

    let dl = l2 - l1;
    let dc = c2p - c1p;

    let dh_deg = if c1p * c2p == 0.0 {
        0.0
    } else if (h2p - h1p).abs() <= 180.0 {
        h2p - h1p
    } else if h2p - h1p > 180.0 {
        h2p - h1p - 360.0
    } else {
        h2p - h1p + 360.0
    };

    let dh = 2.0 * (c1p * c2p).sqrt() * (dh_deg * PI / 360.0).sin();

    let l_avg = (l1 + l2) / 2.0;
    let c_avg_p = (c1p + c2p) / 2.0;

    let h_avg = if c1p * c2p == 0.0 {
        h1p + h2p
    } else if (h1p - h2p).abs() <= 180.0 {
        (h1p + h2p) / 2.0
    } else if h1p + h2p < 360.0 {
        (h1p + h2p + 360.0) / 2.0
    } else {
        (h1p + h2p - 360.0) / 2.0
    };

    let t = 1.0
        - 0.17 * ((h_avg - 30.0) * PI / 180.0).cos()
        + 0.24 * ((2.0 * h_avg) * PI / 180.0).cos()
        + 0.32 * ((3.0 * h_avg + 6.0) * PI / 180.0).cos()
        - 0.20 * ((4.0 * h_avg - 63.0) * PI / 180.0).cos();

    let sl = 1.0 + 0.015 * (l_avg - 50.0).powi(2) / (20.0 + (l_avg - 50.0).powi(2)).sqrt();
    let sc = 1.0 + 0.045 * c_avg_p;
    let sh = 1.0 + 0.015 * c_avg_p * t;

    let c_avg_p_pow7 = c_avg_p.powi(7);
    let rt = -2.0
        * (c_avg_p_pow7 / (c_avg_p_pow7 + POW25_7)).sqrt()
        * (60.0 * (-((h_avg - 275.0) / 25.0).powi(2)).exp() * PI / 180.0).sin();

    let term_l = dl / sl;
    let term_c = dc / sc;
    let term_h = dh / sh;

    (term_l * term_l + term_c * term_c + term_h * term_h + rt * term_c * term_h).sqrt()
}

/// Find closest thread color matches for a given hex color.
pub fn find_matches(
    hex: &str,
    brands: Option<&[String]>,
    limit: usize,
) -> Vec<ThreadMatch> {
    let hex_clean = hex.trim_start_matches('#');
    if hex_clean.len() != 6 {
        return Vec::new();
    }
    let r = match u8::from_str_radix(&hex_clean[0..2], 16) { Ok(v) => v, Err(_) => return Vec::new() };
    let g = match u8::from_str_radix(&hex_clean[2..4], 16) { Ok(v) => v, Err(_) => return Vec::new() };
    let b = match u8::from_str_radix(&hex_clean[4..6], 16) { Ok(v) => v, Err(_) => return Vec::new() };

    let (l1, a1, b1) = rgb_to_lab(r, g, b);
    let cache = get_lab_cache();

    let mut matches: Vec<ThreadMatch> = cache
        .iter()
        .filter(|entry| {
            if let Some(brands) = brands {
                brands.iter().any(|b| b.eq_ignore_ascii_case(entry.color.brand))
            } else {
                true
            }
        })
        .map(|entry| {
            let de = ciede2000(l1, a1, b1, entry.l, entry.a, entry.b);
            ThreadMatch {
                brand: entry.color.brand.to_string(),
                code: entry.color.code.to_string(),
                name: entry.color.name.to_string(),
                hex: format!(
                    "#{:02X}{:02X}{:02X}",
                    entry.color.r, entry.color.g, entry.color.b
                ),
                delta_e: (de * 100.0).round() / 100.0,
            }
        })
        .collect();

    matches.sort_by(|a, b| a.delta_e.partial_cmp(&b.delta_e).unwrap_or(std::cmp::Ordering::Equal));
    matches.truncate(limit);
    matches
}

/// Get all available brand names (derived from the color database, cached).
pub fn available_brands() -> Vec<String> {
    static BRANDS: OnceLock<Vec<String>> = OnceLock::new();
    BRANDS.get_or_init(|| {
        all_thread_colors()
            .iter()
            .map(|c| c.brand.to_string())
            .collect::<std::collections::BTreeSet<_>>()
            .into_iter()
            .collect()
    }).clone()
}

/// Return all thread colors in the static database.
fn all_thread_colors() -> &'static [ThreadBrandColor] {
    static COLORS: OnceLock<Vec<ThreadBrandColor>> = OnceLock::new();
    COLORS.get_or_init(|| {
        let mut all = Vec::with_capacity(2000);
        all.extend_from_slice(MADEIRA_COLORS);
        all.extend_from_slice(ISACORD_COLORS);
        all.extend_from_slice(BROTHER_COLORS);
        all.extend_from_slice(JANOME_COLORS);
        all.extend_from_slice(SULKY_COLORS);
        all.extend_from_slice(ROBISON_ANTON_COLORS);
        all.extend_from_slice(GUNOLD_COLORS);
        all
    })
}

/// Get all colors for a specific brand.
pub fn brand_colors(brand: &str) -> Vec<ThreadBrandColor> {
    all_thread_colors()
        .iter()
        .filter(|c| c.brand.eq_ignore_ascii_case(brand))
        .cloned()
        .collect()
}

// ─── Thread Color Databases ───────────────────────────────────────────────────
// Data sourced from publicly available manufacturer thread charts.

const MADEIRA_COLORS: &[ThreadBrandColor] = &[
    ThreadBrandColor { brand: "Madeira", code: "1000", name: "Rayon Black", r: 0, g: 0, b: 0 },
    ThreadBrandColor { brand: "Madeira", code: "1001", name: "White", r: 255, g: 255, b: 255 },
    ThreadBrandColor { brand: "Madeira", code: "1037", name: "Fruit Punch", r: 237, g: 28, b: 36 },
    ThreadBrandColor { brand: "Madeira", code: "1039", name: "Brick Red", r: 189, g: 33, b: 48 },
    ThreadBrandColor { brand: "Madeira", code: "1040", name: "Magenta", r: 200, g: 16, b: 70 },
    ThreadBrandColor { brand: "Madeira", code: "1046", name: "Christmas Red", r: 197, g: 29, b: 52 },
    ThreadBrandColor { brand: "Madeira", code: "1047", name: "Persimmon", r: 255, g: 75, b: 38 },
    ThreadBrandColor { brand: "Madeira", code: "1058", name: "Rust", r: 176, g: 63, b: 40 },
    ThreadBrandColor { brand: "Madeira", code: "1078", name: "Tangerine", r: 255, g: 130, b: 40 },
    ThreadBrandColor { brand: "Madeira", code: "1065", name: "Burnt Orange", r: 204, g: 85, b: 0 },
    ThreadBrandColor { brand: "Madeira", code: "1024", name: "Goldenrod", r: 254, g: 186, b: 53 },
    ThreadBrandColor { brand: "Madeira", code: "1023", name: "Lemon", r: 255, g: 233, b: 0 },
    ThreadBrandColor { brand: "Madeira", code: "1100", name: "Yellow", r: 255, g: 255, b: 0 },
    ThreadBrandColor { brand: "Madeira", code: "1167", name: "Ivy", r: 0, g: 103, b: 62 },
    ThreadBrandColor { brand: "Madeira", code: "1170", name: "Evergreen", r: 0, g: 80, b: 40 },
    ThreadBrandColor { brand: "Madeira", code: "1177", name: "Meadow", r: 60, g: 160, b: 60 },
    ThreadBrandColor { brand: "Madeira", code: "1180", name: "Lime", r: 112, g: 188, b: 31 },
    ThreadBrandColor { brand: "Madeira", code: "1280", name: "Emerald", r: 0, g: 135, b: 90 },
    ThreadBrandColor { brand: "Madeira", code: "1091", name: "Moss Green", r: 80, g: 120, b: 50 },
    ThreadBrandColor { brand: "Madeira", code: "1132", name: "Mint Green", r: 158, g: 214, b: 125 },
    ThreadBrandColor { brand: "Madeira", code: "1146", name: "Forest Green", r: 34, g: 90, b: 34 },
    ThreadBrandColor { brand: "Madeira", code: "1075", name: "Peach", r: 255, g: 190, b: 150 },
    ThreadBrandColor { brand: "Madeira", code: "1108", name: "Flesh", r: 240, g: 195, b: 175 },
    ThreadBrandColor { brand: "Madeira", code: "1120", name: "Salmon", r: 250, g: 128, b: 114 },
    ThreadBrandColor { brand: "Madeira", code: "1116", name: "Pink", r: 249, g: 147, b: 188 },
    ThreadBrandColor { brand: "Madeira", code: "1117", name: "Carnation", r: 255, g: 105, b: 140 },
    ThreadBrandColor { brand: "Madeira", code: "1119", name: "Fuchsia", r: 200, g: 0, b: 100 },
    ThreadBrandColor { brand: "Madeira", code: "1311", name: "Dusty Rose", r: 190, g: 110, b: 120 },
    ThreadBrandColor { brand: "Madeira", code: "1112", name: "Light Pink", r: 255, g: 200, b: 210 },
    ThreadBrandColor { brand: "Madeira", code: "1320", name: "Burgundy", r: 128, g: 0, b: 32 },
    ThreadBrandColor { brand: "Madeira", code: "1174", name: "Teal", r: 0, g: 128, b: 128 },
    ThreadBrandColor { brand: "Madeira", code: "1028", name: "Sky Blue", r: 135, g: 206, b: 235 },
    ThreadBrandColor { brand: "Madeira", code: "1029", name: "Blue", r: 10, g: 85, b: 163 },
    ThreadBrandColor { brand: "Madeira", code: "1030", name: "Prussian Blue", r: 14, g: 31, b: 124 },
    ThreadBrandColor { brand: "Madeira", code: "1031", name: "Navy", r: 0, g: 0, b: 128 },
    ThreadBrandColor { brand: "Madeira", code: "1033", name: "Royal Blue", r: 65, g: 105, b: 225 },
    ThreadBrandColor { brand: "Madeira", code: "1076", name: "Light Blue", r: 168, g: 222, b: 235 },
    ThreadBrandColor { brand: "Madeira", code: "1310", name: "Lilac", r: 178, g: 175, b: 212 },
    ThreadBrandColor { brand: "Madeira", code: "1313", name: "Purple", r: 119, g: 1, b: 118 },
    ThreadBrandColor { brand: "Madeira", code: "1312", name: "Violet", r: 106, g: 28, b: 138 },
    ThreadBrandColor { brand: "Madeira", code: "1055", name: "Dark Brown", r: 80, g: 40, b: 20 },
    ThreadBrandColor { brand: "Madeira", code: "1057", name: "Light Brown", r: 178, g: 118, b: 36 },
    ThreadBrandColor { brand: "Madeira", code: "1060", name: "Chocolate", r: 100, g: 50, b: 20 },
    ThreadBrandColor { brand: "Madeira", code: "1128", name: "Beige", r: 239, g: 227, b: 185 },
    ThreadBrandColor { brand: "Madeira", code: "1340", name: "Gray", r: 150, g: 150, b: 150 },
    ThreadBrandColor { brand: "Madeira", code: "1341", name: "Silver", r: 192, g: 192, b: 192 },
    ThreadBrandColor { brand: "Madeira", code: "1342", name: "Pewter", r: 100, g: 100, b: 100 },
    ThreadBrandColor { brand: "Madeira", code: "1344", name: "Charcoal", r: 60, g: 60, b: 60 },
    ThreadBrandColor { brand: "Madeira", code: "1070", name: "Gold", r: 218, g: 165, b: 32 },
    ThreadBrandColor { brand: "Madeira", code: "1025", name: "Deep Gold", r: 186, g: 152, b: 0 },
];

const ISACORD_COLORS: &[ThreadBrandColor] = &[
    ThreadBrandColor { brand: "Isacord", code: "0020", name: "Black", r: 0, g: 0, b: 0 },
    ThreadBrandColor { brand: "Isacord", code: "0010", name: "Silky White", r: 255, g: 255, b: 255 },
    ThreadBrandColor { brand: "Isacord", code: "0015", name: "White", r: 250, g: 250, b: 250 },
    ThreadBrandColor { brand: "Isacord", code: "1702", name: "Red Berry", r: 155, g: 25, b: 50 },
    ThreadBrandColor { brand: "Isacord", code: "1800", name: "Wildfire", r: 210, g: 30, b: 30 },
    ThreadBrandColor { brand: "Isacord", code: "1805", name: "Strawberry", r: 220, g: 25, b: 50 },
    ThreadBrandColor { brand: "Isacord", code: "1900", name: "Geranium", r: 235, g: 25, b: 40 },
    ThreadBrandColor { brand: "Isacord", code: "1902", name: "Lipstick", r: 200, g: 20, b: 60 },
    ThreadBrandColor { brand: "Isacord", code: "1904", name: "Cardinal", r: 180, g: 15, b: 40 },
    ThreadBrandColor { brand: "Isacord", code: "2101", name: "Country Red", r: 190, g: 40, b: 40 },
    ThreadBrandColor { brand: "Isacord", code: "2300", name: "Bright Ruby", r: 170, g: 0, b: 50 },
    ThreadBrandColor { brand: "Isacord", code: "2500", name: "Dusty Rose", r: 185, g: 110, b: 115 },
    ThreadBrandColor { brand: "Isacord", code: "2560", name: "Orchid", r: 180, g: 70, b: 120 },
    ThreadBrandColor { brand: "Isacord", code: "2600", name: "Dusty Grape", r: 130, g: 60, b: 100 },
    ThreadBrandColor { brand: "Isacord", code: "2702", name: "Grape", r: 100, g: 30, b: 90 },
    ThreadBrandColor { brand: "Isacord", code: "2905", name: "Iris", r: 80, g: 50, b: 130 },
    ThreadBrandColor { brand: "Isacord", code: "2920", name: "Purple", r: 100, g: 20, b: 130 },
    ThreadBrandColor { brand: "Isacord", code: "3344", name: "Lobelia", r: 60, g: 60, b: 140 },
    ThreadBrandColor { brand: "Isacord", code: "3543", name: "Royal Blue", r: 20, g: 60, b: 170 },
    ThreadBrandColor { brand: "Isacord", code: "3600", name: "Nordic Blue", r: 15, g: 45, b: 130 },
    ThreadBrandColor { brand: "Isacord", code: "3622", name: "Imperial Blue", r: 10, g: 30, b: 100 },
    ThreadBrandColor { brand: "Isacord", code: "3732", name: "Slate Blue", r: 80, g: 80, b: 120 },
    ThreadBrandColor { brand: "Isacord", code: "3815", name: "Reef Blue", r: 30, g: 100, b: 160 },
    ThreadBrandColor { brand: "Isacord", code: "3910", name: "Crystal Blue", r: 100, g: 170, b: 220 },
    ThreadBrandColor { brand: "Isacord", code: "3951", name: "Azure", r: 140, g: 200, b: 230 },
    ThreadBrandColor { brand: "Isacord", code: "4101", name: "Jade", r: 0, g: 120, b: 100 },
    ThreadBrandColor { brand: "Isacord", code: "4220", name: "Teal", r: 0, g: 130, b: 120 },
    ThreadBrandColor { brand: "Isacord", code: "4625", name: "Shamrock", r: 0, g: 150, b: 70 },
    ThreadBrandColor { brand: "Isacord", code: "5326", name: "Evergreen", r: 0, g: 80, b: 45 },
    ThreadBrandColor { brand: "Isacord", code: "5422", name: "Swiss Ivy", r: 60, g: 130, b: 60 },
    ThreadBrandColor { brand: "Isacord", code: "5510", name: "Emerald", r: 0, g: 100, b: 60 },
    ThreadBrandColor { brand: "Isacord", code: "5531", name: "Lime", r: 130, g: 190, b: 40 },
    ThreadBrandColor { brand: "Isacord", code: "5833", name: "Lime Green", r: 100, g: 175, b: 50 },
    ThreadBrandColor { brand: "Isacord", code: "5912", name: "Erin Green", r: 80, g: 160, b: 50 },
    ThreadBrandColor { brand: "Isacord", code: "6010", name: "Grass Green", r: 60, g: 140, b: 30 },
    ThreadBrandColor { brand: "Isacord", code: "0722", name: "Khaki", r: 175, g: 155, b: 120 },
    ThreadBrandColor { brand: "Isacord", code: "0761", name: "Oat", r: 200, g: 180, b: 150 },
    ThreadBrandColor { brand: "Isacord", code: "0853", name: "Pecan", r: 140, g: 80, b: 40 },
    ThreadBrandColor { brand: "Isacord", code: "0945", name: "Pine Bark", r: 100, g: 60, b: 30 },
    ThreadBrandColor { brand: "Isacord", code: "1055", name: "Bark", r: 80, g: 50, b: 25 },
    ThreadBrandColor { brand: "Isacord", code: "1134", name: "Light Cocoa", r: 160, g: 110, b: 80 },
    ThreadBrandColor { brand: "Isacord", code: "0520", name: "Daffodil", r: 255, g: 210, b: 0 },
    ThreadBrandColor { brand: "Isacord", code: "0600", name: "Citrus", r: 255, g: 200, b: 0 },
    ThreadBrandColor { brand: "Isacord", code: "0501", name: "Sun", r: 255, g: 230, b: 0 },
    ThreadBrandColor { brand: "Isacord", code: "0700", name: "Bright Yellow", r: 255, g: 255, b: 0 },
    ThreadBrandColor { brand: "Isacord", code: "0800", name: "Golden Rod", r: 218, g: 165, b: 32 },
    ThreadBrandColor { brand: "Isacord", code: "0940", name: "Autumn Leaf", r: 200, g: 100, b: 30 },
    ThreadBrandColor { brand: "Isacord", code: "1010", name: "Tangerine", r: 250, g: 130, b: 40 },
    ThreadBrandColor { brand: "Isacord", code: "1102", name: "Paprika", r: 220, g: 60, b: 20 },
    ThreadBrandColor { brand: "Isacord", code: "0108", name: "Cobblestone", r: 160, g: 160, b: 160 },
    ThreadBrandColor { brand: "Isacord", code: "0111", name: "Oyster", r: 200, g: 195, b: 180 },
    ThreadBrandColor { brand: "Isacord", code: "0124", name: "Fieldstone", r: 130, g: 130, b: 130 },
    ThreadBrandColor { brand: "Isacord", code: "0131", name: "Smoke", r: 100, g: 100, b: 100 },
    ThreadBrandColor { brand: "Isacord", code: "0142", name: "Sterling", r: 190, g: 190, b: 190 },
    ThreadBrandColor { brand: "Isacord", code: "0150", name: "Mystik Grey", r: 70, g: 70, b: 70 },
    ThreadBrandColor { brand: "Isacord", code: "2150", name: "Petal Pink", r: 245, g: 190, b: 200 },
    ThreadBrandColor { brand: "Isacord", code: "2152", name: "Pink", r: 245, g: 150, b: 175 },
    ThreadBrandColor { brand: "Isacord", code: "2153", name: "Dusty Mauve", r: 180, g: 120, b: 130 },
    ThreadBrandColor { brand: "Isacord", code: "2220", name: "Light Rose", r: 230, g: 150, b: 165 },
    ThreadBrandColor { brand: "Isacord", code: "2520", name: "Garden Rose", r: 200, g: 80, b: 100 },
];

const BROTHER_COLORS: &[ThreadBrandColor] = &[
    ThreadBrandColor { brand: "Brother", code: "000", name: "Unknown", r: 0, g: 0, b: 0 },
    ThreadBrandColor { brand: "Brother", code: "007", name: "Prussian Blue", r: 14, g: 31, b: 124 },
    ThreadBrandColor { brand: "Brother", code: "405", name: "Blue", r: 10, g: 85, b: 163 },
    ThreadBrandColor { brand: "Brother", code: "534", name: "Teal Green", r: 0, g: 135, b: 119 },
    ThreadBrandColor { brand: "Brother", code: "070", name: "Cornflower Blue", r: 75, g: 107, b: 175 },
    ThreadBrandColor { brand: "Brother", code: "800", name: "Red", r: 237, g: 23, b: 31 },
    ThreadBrandColor { brand: "Brother", code: "058", name: "Reddish Brown", r: 209, g: 92, b: 0 },
    ThreadBrandColor { brand: "Brother", code: "614", name: "Magenta", r: 145, g: 54, b: 151 },
    ThreadBrandColor { brand: "Brother", code: "804", name: "Light Lilac", r: 228, g: 154, b: 203 },
    ThreadBrandColor { brand: "Brother", code: "612", name: "Lilac", r: 145, g: 95, b: 172 },
    ThreadBrandColor { brand: "Brother", code: "502", name: "Mint Green", r: 158, g: 214, b: 125 },
    ThreadBrandColor { brand: "Brother", code: "214", name: "Deep Gold", r: 232, g: 169, b: 0 },
    ThreadBrandColor { brand: "Brother", code: "208", name: "Orange", r: 254, g: 186, b: 53 },
    ThreadBrandColor { brand: "Brother", code: "205", name: "Yellow", r: 255, g: 255, b: 0 },
    ThreadBrandColor { brand: "Brother", code: "513", name: "Lime Green", r: 112, g: 188, b: 31 },
    ThreadBrandColor { brand: "Brother", code: "328", name: "Brass", r: 186, g: 152, b: 0 },
    ThreadBrandColor { brand: "Brother", code: "005", name: "Silver", r: 168, g: 168, b: 168 },
    ThreadBrandColor { brand: "Brother", code: "337", name: "Russet Brown", r: 125, g: 111, b: 0 },
    ThreadBrandColor { brand: "Brother", code: "101", name: "Cream Brown", r: 255, g: 255, b: 179 },
    ThreadBrandColor { brand: "Brother", code: "843", name: "Pewter", r: 79, g: 85, b: 86 },
    ThreadBrandColor { brand: "Brother", code: "900", name: "Black", r: 0, g: 0, b: 0 },
    ThreadBrandColor { brand: "Brother", code: "406", name: "Ultramarine", r: 11, g: 61, b: 145 },
    ThreadBrandColor { brand: "Brother", code: "869", name: "Royal Purple", r: 119, g: 1, b: 118 },
    ThreadBrandColor { brand: "Brother", code: "707", name: "Dark Gray", r: 41, g: 49, b: 51 },
    ThreadBrandColor { brand: "Brother", code: "457", name: "Dark Brown", r: 42, g: 19, b: 1 },
    ThreadBrandColor { brand: "Brother", code: "086", name: "Deep Rose", r: 246, g: 74, b: 138 },
    ThreadBrandColor { brand: "Brother", code: "323", name: "Light Brown", r: 178, g: 118, b: 36 },
    ThreadBrandColor { brand: "Brother", code: "079", name: "Salmon Pink", r: 252, g: 187, b: 197 },
    ThreadBrandColor { brand: "Brother", code: "030", name: "Vermilion", r: 254, g: 55, b: 15 },
    ThreadBrandColor { brand: "Brother", code: "001", name: "White", r: 240, g: 240, b: 240 },
    ThreadBrandColor { brand: "Brother", code: "613", name: "Violet", r: 106, g: 28, b: 138 },
    ThreadBrandColor { brand: "Brother", code: "542", name: "Seacrest", r: 168, g: 221, b: 196 },
    ThreadBrandColor { brand: "Brother", code: "019", name: "Sky Blue", r: 37, g: 132, b: 187 },
    ThreadBrandColor { brand: "Brother", code: "209", name: "Pumpkin", r: 254, g: 179, b: 67 },
    ThreadBrandColor { brand: "Brother", code: "204", name: "Cream Yellow", r: 255, g: 243, b: 107 },
    ThreadBrandColor { brand: "Brother", code: "339", name: "Khaki", r: 208, g: 166, b: 96 },
    ThreadBrandColor { brand: "Brother", code: "349", name: "Clay Brown", r: 209, g: 84, b: 0 },
    ThreadBrandColor { brand: "Brother", code: "515", name: "Leaf Green", r: 102, g: 186, b: 73 },
    ThreadBrandColor { brand: "Brother", code: "507", name: "Peacock Blue", r: 19, g: 74, b: 70 },
    ThreadBrandColor { brand: "Brother", code: "704", name: "Gray", r: 135, g: 135, b: 135 },
    ThreadBrandColor { brand: "Brother", code: "399", name: "Warm Gray", r: 216, g: 204, b: 198 },
    ThreadBrandColor { brand: "Brother", code: "517", name: "Dark Olive", r: 67, g: 86, b: 7 },
    ThreadBrandColor { brand: "Brother", code: "107", name: "Flesh Pink", r: 253, g: 217, b: 222 },
    ThreadBrandColor { brand: "Brother", code: "085", name: "Pink", r: 249, g: 147, b: 188 },
    ThreadBrandColor { brand: "Brother", code: "509", name: "Deep Green", r: 0, g: 56, b: 34 },
    ThreadBrandColor { brand: "Brother", code: "810", name: "Lavender", r: 178, g: 175, b: 212 },
    ThreadBrandColor { brand: "Brother", code: "607", name: "Wisteria Blue", r: 104, g: 106, b: 176 },
    ThreadBrandColor { brand: "Brother", code: "841", name: "Beige", r: 239, g: 227, b: 185 },
    ThreadBrandColor { brand: "Brother", code: "807", name: "Carmine", r: 247, g: 56, b: 102 },
    ThreadBrandColor { brand: "Brother", code: "333", name: "Amber Red", r: 181, g: 75, b: 100 },
];

const JANOME_COLORS: &[ThreadBrandColor] = &[
    ThreadBrandColor { brand: "Janome", code: "01", name: "Black", r: 0, g: 0, b: 0 },
    ThreadBrandColor { brand: "Janome", code: "02", name: "White", r: 255, g: 255, b: 255 },
    ThreadBrandColor { brand: "Janome", code: "03", name: "Yellow", r: 255, g: 255, b: 23 },
    ThreadBrandColor { brand: "Janome", code: "04", name: "Orange", r: 255, g: 140, b: 0 },
    ThreadBrandColor { brand: "Janome", code: "05", name: "Red", r: 255, g: 0, b: 0 },
    ThreadBrandColor { brand: "Janome", code: "06", name: "Pink", r: 226, g: 72, b: 131 },
    ThreadBrandColor { brand: "Janome", code: "07", name: "Purple", r: 171, g: 90, b: 150 },
    ThreadBrandColor { brand: "Janome", code: "08", name: "Blue", r: 11, g: 47, b: 132 },
    ThreadBrandColor { brand: "Janome", code: "09", name: "Green", r: 26, g: 132, b: 45 },
    ThreadBrandColor { brand: "Janome", code: "10", name: "Pale Yellow", r: 252, g: 242, b: 148 },
    ThreadBrandColor { brand: "Janome", code: "11", name: "Pale Pink", r: 249, g: 153, b: 183 },
    ThreadBrandColor { brand: "Janome", code: "12", name: "Light Blue", r: 56, g: 108, b: 174 },
    ThreadBrandColor { brand: "Janome", code: "13", name: "Yellow Green", r: 127, g: 194, b: 28 },
    ThreadBrandColor { brand: "Janome", code: "14", name: "Vermilion", r: 240, g: 51, b: 31 },
    ThreadBrandColor { brand: "Janome", code: "15", name: "Coral", r: 249, g: 103, b: 107 },
    ThreadBrandColor { brand: "Janome", code: "16", name: "Emerald Green", r: 76, g: 191, b: 143 },
    ThreadBrandColor { brand: "Janome", code: "17", name: "Crimson", r: 243, g: 54, b: 137 },
    ThreadBrandColor { brand: "Janome", code: "18", name: "Brown", r: 80, g: 50, b: 20 },
    ThreadBrandColor { brand: "Janome", code: "19", name: "Gray", r: 155, g: 155, b: 155 },
    ThreadBrandColor { brand: "Janome", code: "20", name: "Navy", r: 0, g: 0, b: 128 },
    ThreadBrandColor { brand: "Janome", code: "21", name: "Dark Green", r: 0, g: 128, b: 0 },
    ThreadBrandColor { brand: "Janome", code: "22", name: "Maroon", r: 128, g: 0, b: 0 },
    ThreadBrandColor { brand: "Janome", code: "23", name: "Gold", r: 255, g: 215, b: 0 },
    ThreadBrandColor { brand: "Janome", code: "24", name: "Silver", r: 192, g: 192, b: 192 },
    ThreadBrandColor { brand: "Janome", code: "25", name: "Sky Blue", r: 135, g: 206, b: 235 },
    ThreadBrandColor { brand: "Janome", code: "26", name: "Light Pink", r: 255, g: 182, b: 193 },
    ThreadBrandColor { brand: "Janome", code: "27", name: "Lime Green", r: 0, g: 255, b: 0 },
    ThreadBrandColor { brand: "Janome", code: "28", name: "Deep Purple", r: 128, g: 0, b: 128 },
    ThreadBrandColor { brand: "Janome", code: "29", name: "Turquoise", r: 0, g: 206, b: 209 },
    ThreadBrandColor { brand: "Janome", code: "30", name: "Bright Orange", r: 255, g: 69, b: 0 },
];

const SULKY_COLORS: &[ThreadBrandColor] = &[
    ThreadBrandColor { brand: "Sulky", code: "1001", name: "Bright White", r: 255, g: 255, b: 255 },
    ThreadBrandColor { brand: "Sulky", code: "1002", name: "Soft White", r: 250, g: 248, b: 240 },
    ThreadBrandColor { brand: "Sulky", code: "1005", name: "Black", r: 0, g: 0, b: 0 },
    ThreadBrandColor { brand: "Sulky", code: "1147", name: "Christmas Red", r: 200, g: 20, b: 30 },
    ThreadBrandColor { brand: "Sulky", code: "1037", name: "Light Red", r: 237, g: 60, b: 60 },
    ThreadBrandColor { brand: "Sulky", code: "1039", name: "True Red", r: 220, g: 20, b: 30 },
    ThreadBrandColor { brand: "Sulky", code: "1181", name: "Rust", r: 180, g: 70, b: 35 },
    ThreadBrandColor { brand: "Sulky", code: "1168", name: "Red Orange", r: 230, g: 60, b: 20 },
    ThreadBrandColor { brand: "Sulky", code: "1078", name: "Tangerine", r: 250, g: 130, b: 40 },
    ThreadBrandColor { brand: "Sulky", code: "1024", name: "Goldenrod", r: 220, g: 170, b: 40 },
    ThreadBrandColor { brand: "Sulky", code: "1023", name: "Yellow", r: 255, g: 235, b: 0 },
    ThreadBrandColor { brand: "Sulky", code: "1124", name: "Sun Yellow", r: 255, g: 255, b: 0 },
    ThreadBrandColor { brand: "Sulky", code: "1049", name: "Pale Yellow", r: 255, g: 245, b: 140 },
    ThreadBrandColor { brand: "Sulky", code: "1177", name: "Avocado", r: 80, g: 140, b: 50 },
    ThreadBrandColor { brand: "Sulky", code: "1176", name: "Medium Dark Avocado", r: 50, g: 100, b: 30 },
    ThreadBrandColor { brand: "Sulky", code: "1175", name: "Dark Avocado", r: 30, g: 75, b: 20 },
    ThreadBrandColor { brand: "Sulky", code: "1051", name: "Christmas Green", r: 0, g: 100, b: 50 },
    ThreadBrandColor { brand: "Sulky", code: "1232", name: "Classic Green", r: 0, g: 130, b: 70 },
    ThreadBrandColor { brand: "Sulky", code: "1510", name: "Lime Green", r: 120, g: 190, b: 40 },
    ThreadBrandColor { brand: "Sulky", code: "1205", name: "Mint Green", r: 160, g: 220, b: 140 },
    ThreadBrandColor { brand: "Sulky", code: "1534", name: "Sapphire", r: 20, g: 60, b: 150 },
    ThreadBrandColor { brand: "Sulky", code: "1535", name: "Team Blue", r: 10, g: 40, b: 120 },
    ThreadBrandColor { brand: "Sulky", code: "1076", name: "Royal Blue", r: 50, g: 80, b: 180 },
    ThreadBrandColor { brand: "Sulky", code: "1196", name: "Blue Ribbon", r: 30, g: 90, b: 170 },
    ThreadBrandColor { brand: "Sulky", code: "1029", name: "Medium Blue", r: 40, g: 100, b: 180 },
    ThreadBrandColor { brand: "Sulky", code: "1028", name: "Baby Blue", r: 135, g: 200, b: 235 },
    ThreadBrandColor { brand: "Sulky", code: "1030", name: "Periwinkle", r: 130, g: 130, b: 200 },
    ThreadBrandColor { brand: "Sulky", code: "1031", name: "Medium Orchid", r: 140, g: 80, b: 160 },
    ThreadBrandColor { brand: "Sulky", code: "1032", name: "Med Purple", r: 100, g: 30, b: 120 },
    ThreadBrandColor { brand: "Sulky", code: "1033", name: "Dark Purple", r: 70, g: 10, b: 100 },
    ThreadBrandColor { brand: "Sulky", code: "1119", name: "Dark Rose", r: 190, g: 20, b: 70 },
    ThreadBrandColor { brand: "Sulky", code: "1117", name: "Mauve", r: 180, g: 100, b: 120 },
    ThreadBrandColor { brand: "Sulky", code: "1115", name: "Light Pink", r: 250, g: 200, b: 210 },
    ThreadBrandColor { brand: "Sulky", code: "1108", name: "Flesh", r: 240, g: 195, b: 170 },
    ThreadBrandColor { brand: "Sulky", code: "1128", name: "Dark Ecru", r: 210, g: 190, b: 150 },
    ThreadBrandColor { brand: "Sulky", code: "1149", name: "Deep Ecru", r: 190, g: 165, b: 120 },
    ThreadBrandColor { brand: "Sulky", code: "1055", name: "Tawny Tan", r: 160, g: 110, b: 70 },
    ThreadBrandColor { brand: "Sulky", code: "1057", name: "Dark Tawny Tan", r: 130, g: 80, b: 45 },
    ThreadBrandColor { brand: "Sulky", code: "1130", name: "Dark Brown", r: 80, g: 40, b: 20 },
    ThreadBrandColor { brand: "Sulky", code: "1220", name: "Charcoal", r: 60, g: 60, b: 60 },
    ThreadBrandColor { brand: "Sulky", code: "1219", name: "Gray", r: 140, g: 140, b: 140 },
    ThreadBrandColor { brand: "Sulky", code: "1218", name: "Silver", r: 192, g: 192, b: 192 },
    ThreadBrandColor { brand: "Sulky", code: "1071", name: "Off White", r: 245, g: 240, b: 225 },
    ThreadBrandColor { brand: "Sulky", code: "1174", name: "Dk Teal", r: 0, g: 100, b: 100 },
    ThreadBrandColor { brand: "Sulky", code: "1095", name: "Turquoise", r: 0, g: 160, b: 170 },
];

const ROBISON_ANTON_COLORS: &[ThreadBrandColor] = &[
    ThreadBrandColor { brand: "Robison-Anton", code: "5502", name: "Pro White", r: 255, g: 255, b: 255 },
    ThreadBrandColor { brand: "Robison-Anton", code: "5596", name: "TH Black", r: 0, g: 0, b: 0 },
    ThreadBrandColor { brand: "Robison-Anton", code: "5700", name: "Red", r: 230, g: 25, b: 30 },
    ThreadBrandColor { brand: "Robison-Anton", code: "5724", name: "Cardinal", r: 180, g: 20, b: 40 },
    ThreadBrandColor { brand: "Robison-Anton", code: "5543", name: "Cranberry", r: 150, g: 20, b: 50 },
    ThreadBrandColor { brand: "Robison-Anton", code: "5807", name: "Wildfire", r: 220, g: 40, b: 30 },
    ThreadBrandColor { brand: "Robison-Anton", code: "5563", name: "Burgundy", r: 128, g: 0, b: 32 },
    ThreadBrandColor { brand: "Robison-Anton", code: "5809", name: "Hot Pink", r: 235, g: 50, b: 120 },
    ThreadBrandColor { brand: "Robison-Anton", code: "5733", name: "Pink", r: 245, g: 140, b: 175 },
    ThreadBrandColor { brand: "Robison-Anton", code: "5557", name: "Light Pink", r: 250, g: 200, b: 210 },
    ThreadBrandColor { brand: "Robison-Anton", code: "5732", name: "Dusty Rose", r: 185, g: 110, b: 120 },
    ThreadBrandColor { brand: "Robison-Anton", code: "5726", name: "Purple", r: 100, g: 20, b: 120 },
    ThreadBrandColor { brand: "Robison-Anton", code: "5613", name: "Violet", r: 80, g: 40, b: 130 },
    ThreadBrandColor { brand: "Robison-Anton", code: "5550", name: "Lavender", r: 170, g: 160, b: 210 },
    ThreadBrandColor { brand: "Robison-Anton", code: "5687", name: "Navy", r: 0, g: 0, b: 100 },
    ThreadBrandColor { brand: "Robison-Anton", code: "5735", name: "Royal Blue", r: 30, g: 60, b: 170 },
    ThreadBrandColor { brand: "Robison-Anton", code: "5551", name: "Medium Blue", r: 50, g: 100, b: 180 },
    ThreadBrandColor { brand: "Robison-Anton", code: "5559", name: "Baby Blue", r: 140, g: 200, b: 230 },
    ThreadBrandColor { brand: "Robison-Anton", code: "5564", name: "Teal", r: 0, g: 128, b: 128 },
    ThreadBrandColor { brand: "Robison-Anton", code: "5722", name: "Hunter Green", r: 0, g: 80, b: 40 },
    ThreadBrandColor { brand: "Robison-Anton", code: "5549", name: "Emerald", r: 0, g: 120, b: 70 },
    ThreadBrandColor { brand: "Robison-Anton", code: "5610", name: "Lime", r: 120, g: 190, b: 40 },
    ThreadBrandColor { brand: "Robison-Anton", code: "5756", name: "Bright Green", r: 0, g: 180, b: 60 },
    ThreadBrandColor { brand: "Robison-Anton", code: "5723", name: "Forest Green", r: 30, g: 80, b: 30 },
    ThreadBrandColor { brand: "Robison-Anton", code: "5607", name: "Yellow", r: 255, g: 255, b: 0 },
    ThreadBrandColor { brand: "Robison-Anton", code: "5727", name: "Gold", r: 218, g: 165, b: 32 },
    ThreadBrandColor { brand: "Robison-Anton", code: "5730", name: "Orange", r: 250, g: 130, b: 40 },
    ThreadBrandColor { brand: "Robison-Anton", code: "5737", name: "Rust", r: 180, g: 70, b: 30 },
    ThreadBrandColor { brand: "Robison-Anton", code: "5704", name: "Brown", r: 100, g: 60, b: 30 },
    ThreadBrandColor { brand: "Robison-Anton", code: "5719", name: "Dark Brown", r: 60, g: 30, b: 15 },
    ThreadBrandColor { brand: "Robison-Anton", code: "5554", name: "Taupe", r: 175, g: 155, b: 130 },
    ThreadBrandColor { brand: "Robison-Anton", code: "5770", name: "Gray", r: 140, g: 140, b: 140 },
    ThreadBrandColor { brand: "Robison-Anton", code: "5601", name: "Silver", r: 192, g: 192, b: 192 },
    ThreadBrandColor { brand: "Robison-Anton", code: "5743", name: "Charcoal", r: 60, g: 60, b: 60 },
    ThreadBrandColor { brand: "Robison-Anton", code: "5547", name: "Ecru", r: 235, g: 225, b: 200 },
];

const GUNOLD_COLORS: &[ThreadBrandColor] = &[
    ThreadBrandColor { brand: "Gunold", code: "1001", name: "White", r: 255, g: 255, b: 255 },
    ThreadBrandColor { brand: "Gunold", code: "1005", name: "Black", r: 0, g: 0, b: 0 },
    ThreadBrandColor { brand: "Gunold", code: "1147", name: "Christmas Red", r: 200, g: 20, b: 30 },
    ThreadBrandColor { brand: "Gunold", code: "1037", name: "Bright Red", r: 237, g: 30, b: 35 },
    ThreadBrandColor { brand: "Gunold", code: "1039", name: "True Red", r: 220, g: 20, b: 35 },
    ThreadBrandColor { brand: "Gunold", code: "1181", name: "Rust", r: 180, g: 65, b: 35 },
    ThreadBrandColor { brand: "Gunold", code: "1168", name: "Red Orange", r: 230, g: 55, b: 20 },
    ThreadBrandColor { brand: "Gunold", code: "1078", name: "Tangerine", r: 250, g: 125, b: 40 },
    ThreadBrandColor { brand: "Gunold", code: "1024", name: "Goldenrod", r: 218, g: 170, b: 40 },
    ThreadBrandColor { brand: "Gunold", code: "1023", name: "Yellow", r: 255, g: 235, b: 0 },
    ThreadBrandColor { brand: "Gunold", code: "1177", name: "Avocado", r: 80, g: 140, b: 50 },
    ThreadBrandColor { brand: "Gunold", code: "1051", name: "Christmas Green", r: 0, g: 100, b: 50 },
    ThreadBrandColor { brand: "Gunold", code: "1232", name: "Emerald Green", r: 0, g: 130, b: 70 },
    ThreadBrandColor { brand: "Gunold", code: "1534", name: "Sapphire", r: 20, g: 60, b: 150 },
    ThreadBrandColor { brand: "Gunold", code: "1535", name: "Dark Blue", r: 10, g: 40, b: 120 },
    ThreadBrandColor { brand: "Gunold", code: "1076", name: "Royal Blue", r: 50, g: 80, b: 180 },
    ThreadBrandColor { brand: "Gunold", code: "1028", name: "Baby Blue", r: 135, g: 200, b: 235 },
    ThreadBrandColor { brand: "Gunold", code: "1032", name: "Purple", r: 100, g: 30, b: 120 },
    ThreadBrandColor { brand: "Gunold", code: "1033", name: "Dark Purple", r: 70, g: 10, b: 100 },
    ThreadBrandColor { brand: "Gunold", code: "1119", name: "Dark Rose", r: 190, g: 20, b: 70 },
    ThreadBrandColor { brand: "Gunold", code: "1115", name: "Light Pink", r: 250, g: 200, b: 210 },
    ThreadBrandColor { brand: "Gunold", code: "1108", name: "Flesh", r: 240, g: 195, b: 170 },
    ThreadBrandColor { brand: "Gunold", code: "1128", name: "Ecru", r: 210, g: 190, b: 150 },
    ThreadBrandColor { brand: "Gunold", code: "1055", name: "Tawny Brown", r: 160, g: 110, b: 70 },
    ThreadBrandColor { brand: "Gunold", code: "1130", name: "Dark Brown", r: 80, g: 40, b: 20 },
    ThreadBrandColor { brand: "Gunold", code: "1220", name: "Charcoal", r: 60, g: 60, b: 60 },
    ThreadBrandColor { brand: "Gunold", code: "1219", name: "Gray", r: 140, g: 140, b: 140 },
    ThreadBrandColor { brand: "Gunold", code: "1218", name: "Silver", r: 192, g: 192, b: 192 },
    ThreadBrandColor { brand: "Gunold", code: "1174", name: "Dark Teal", r: 0, g: 100, b: 100 },
    ThreadBrandColor { brand: "Gunold", code: "1510", name: "Lime Green", r: 120, g: 190, b: 40 },
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rgb_to_lab_black() {
        let (l, a, b) = rgb_to_lab(0, 0, 0);
        assert!(l.abs() < 0.1);
        assert!(a.abs() < 0.1);
        assert!(b.abs() < 0.1);
    }

    #[test]
    fn test_rgb_to_lab_white() {
        let (l, _, _) = rgb_to_lab(255, 255, 255);
        assert!((l - 100.0).abs() < 0.5);
    }

    #[test]
    fn test_ciede2000_identical() {
        let (l, a, b) = rgb_to_lab(128, 64, 32);
        let de = ciede2000(l, a, b, l, a, b);
        assert!(de.abs() < 0.001, "identical colors should have dE~0, got {de}");
    }

    #[test]
    fn test_ciede2000_different() {
        let (l1, a1, b1) = rgb_to_lab(255, 0, 0);
        let (l2, a2, b2) = rgb_to_lab(0, 0, 255);
        let de = ciede2000(l1, a1, b1, l2, a2, b2);
        assert!(de > 30.0, "red vs blue should have large dE, got {de}");
    }

    #[test]
    fn test_find_matches_red() {
        let matches = find_matches("#FF0000", None, 5);
        assert!(!matches.is_empty());
        // First match should have a small delta_e (the closest red)
        assert!(matches[0].delta_e < 15.0, "expected close match for pure red");
    }

    #[test]
    fn test_find_matches_with_brand_filter() {
        let matches = find_matches(
            "#FF0000",
            Some(&["Brother".to_string()]),
            5,
        );
        assert!(!matches.is_empty());
        assert!(matches.iter().all(|m| m.brand == "Brother"));
    }

    #[test]
    fn test_available_brands() {
        let brands = available_brands();
        assert!(brands.contains(&"Madeira".to_string()));
        assert!(brands.contains(&"Isacord".to_string()));
        assert!(brands.contains(&"Brother".to_string()));
        assert!(brands.contains(&"Janome".to_string()));
        assert!(brands.contains(&"Sulky".to_string()));
        assert!(brands.contains(&"Robison-Anton".to_string()));
        assert!(brands.contains(&"Gunold".to_string()));
    }

    #[test]
    fn test_brand_colors() {
        let brother = brand_colors("Brother");
        assert!(brother.len() >= 40, "Brother should have >=40 colors");
        let madeira = brand_colors("Madeira");
        assert!(madeira.len() >= 40, "Madeira should have >=40 colors");
    }

    #[test]
    fn test_all_thread_colors_count() {
        let all = all_thread_colors();
        assert!(all.len() >= 200, "should have at least 200 total colors, got {}", all.len());
    }
}
