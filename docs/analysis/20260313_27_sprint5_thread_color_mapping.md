# Analysis: Issue #30 -- Thread Color Code Mapping

**Date:** 2026-03-13
**Sprint:** 5 (Extended Features)
**Type:** Feature | Effort: L

---

## Problem Description

The application currently parses and displays thread colors from embroidery files (PES, JEF, VP3, DST) with RGB hex values, names, and brand information extracted by the parsers. However, users need to know which **physical thread** to purchase for their embroidery machine. Each manufacturer (Madeira, Isacord, Sulky, Brother, Robison-Anton, Gunold) uses its own proprietary color numbering system.

A file might report a color as `#ED171F "Red"` from the Brother/PEC palette, but a user stitching with Madeira thread needs to know this corresponds to Madeira Rayon 1037 "Fruit Punch" or similar.

Issue #30 requires:
1. Display thread colors alongside corresponding codes from major manufacturers
2. Automatic closest-match suggestions using perceptual color matching (CIE Delta E 2000)
3. Brand filtering by user preference
4. Click-to-search: find files using a specific thread color

---

## Affected Components

### New files
- `src-tauri/src/services/thread_db.rs` -- Static thread color database + CIEDE2000 matching engine
- `src-tauri/src/commands/thread_colors.rs` -- Tauri commands for thread matching
- `src/services/ThreadColorService.ts` -- Frontend invoke wrappers

### Backend modifications
- `src-tauri/Cargo.toml` -- Add `palette` crate dependency
- `src-tauri/src/services/mod.rs` -- Register `thread_db` module
- `src-tauri/src/commands/mod.rs` -- Register `thread_colors` module
- `src-tauri/src/lib.rs` -- Register new Tauri commands in invoke_handler

### Frontend modifications
- `src/types/index.ts` -- Add `ThreadMatch`, `BrandColor` interfaces
- `src/components/MetadataPanel.ts` -- Enhance color swatch rendering with expandable manufacturer matches
- `src/styles/components.css` -- CSS for expanded color swatches with brand matches

### Existing parser palettes (reference, no modification)
- `src-tauri/src/parsers/pes.rs` -- `PEC_PALETTE` (65 Brother colors)
- `src-tauri/src/parsers/jef.rs` -- `JANOME_PALETTE` (78 Janome colors)

### No database migration needed
Thread color database is static Rust data, not stored in SQLite. The existing `file_thread_colors` table with `brand` and `brand_code` columns is sufficient.

---

## Root Cause / Rationale

1. **Practical necessity:** Embroiderers purchase physical thread by manufacturer code number, not RGB hex. Without cross-referencing, users must manually look up each color in printed charts -- tedious for designs with 10+ colors.

2. **Color perception gap:** Simple RGB distance (Euclidean) is perceptually inaccurate. CIE Delta E 2000 is the industry standard for perceptual color difference, essential for accurate thread substitutions.

3. **Existing infrastructure:** Parsers already extract RGB + brand + brand_code for PES (Brother) and JEF (Janome). VP3 extracts RGB + name + brand from file. DST has no color data. MetadataPanel displays color swatches but only shows the single color from the file.

4. **Search integration:** The `color_search` field in `SearchParams` already queries `file_thread_colors.color_name` and `brand`. This can be extended for manufacturer thread code search.

---

## Proposed Approach

### Step 1: Add `palette` crate dependency

```toml
palette = "0.7"
```

Provides CIE L*a*b* color space conversion and CIEDE2000 computation. Far more accurate than Euclidean distance in RGB space.

### Step 2: Create static thread color database (`src-tauri/src/services/thread_db.rs`)

**Data structure:**
```rust
pub struct ThreadBrandColor {
    pub brand: &'static str,
    pub code: &'static str,
    pub name: &'static str,
    pub r: u8,
    pub g: u8,
    pub b: u8,
}
```

**Brand catalogs (priority order):**
- Madeira Rayon (~400 colors) -- most popular globally
- Isacord (~400 colors) -- popular polyester
- Brother (~65 colors, reuse PEC_PALETTE data)
- Janome (~78 colors, reuse JANOME_PALETTE data)
- Sulky (~300 colors) -- popular brand
- Robison-Anton (~250 colors) -- popular in North America
- Gunold (~300 colors) -- major European brand

**Design decisions:**
- Static Rust `const` arrays (not SQLite): thread charts don't change at runtime, zero startup cost, no migration, easily testable
- ~2,000 total colors across all brands -- small enough for static arrays (~100-200KB binary impact)
- Reuse existing `PEC_PALETTE` and `JANOME_PALETTE` data to avoid duplication

### Step 3: Implement CIE Delta E 2000 matching

**Match result struct:**
```rust
pub struct ThreadMatch {
    pub brand: String,
    pub code: String,
    pub name: String,
    pub hex: String,
    pub delta_e: f64,  // 0 = exact match
}
```

**Key functions:**
- `get_thread_matches(hex, brands, limit) -> Vec<ThreadMatch>`: Find closest matches across selected brands, sorted by delta_e
- `get_available_brands() -> Vec<String>`: Return supported brands
- `get_brand_colors(brand) -> Vec<ThreadBrandColor>`: All colors for a brand

**Algorithm:**
1. Parse input hex to `palette::Srgb<u8>`
2. Convert to `palette::Lab` (CIE L*a*b* via D65 illuminant)
3. For each candidate, convert to Lab
4. Compute CIEDE2000 via `palette::color_difference::Ciede2000`
5. Sort by delta_e, return top N

**Performance:** ~2,000 colors brute-force search is microseconds. Pre-compute Lab values using `once_cell::sync::Lazy` to avoid repeated conversions.

### Step 4: Create Tauri commands (`src-tauri/src/commands/thread_colors.rs`)

```rust
#[tauri::command]
pub fn get_thread_matches(color_hex: String, brands: Option<Vec<String>>, limit: Option<usize>) -> Result<Vec<ThreadMatch>, AppError>

#[tauri::command]
pub fn get_available_brands() -> Result<Vec<String>, AppError>

#[tauri::command]
pub fn get_brand_colors(brand: String) -> Result<Vec<BrandColorInfo>, AppError>
```

No database access needed -- these operate on static data.

### Step 5: Create frontend service (`src/services/ThreadColorService.ts`)

Invoke wrappers for the three commands.

### Step 6: Add TypeScript interfaces (`src/types/index.ts`)

```typescript
export interface ThreadMatch {
    brand: string;
    code: string;
    name: string;
    hex: string;
    deltaE: number;
}

export interface BrandColor {
    brand: string;
    code: string;
    name: string;
    hex: string;
}
```

### Step 7: Enhance MetadataPanel color swatch display

Current display (lines ~422-481) shows simple grid with color box, name, brand, hex.

Enhanced design:
- **Expandable swatches:** Each color clickable, expands to show closest matches per brand
- **Lazy loading:** Thread matches fetched on-demand when expanded, not on initial load
- **Brand preference:** Stored in settings (`thread_brands` key), only show preferred brands
- **UI layout:**
  ```
  [#ED171F] Red (Brother #5)
     -- Madeira Rayon 1037 "Fruit Punch" [swatch] dE=1.2
     -- Isacord 1902 "Lipstick"          [swatch] dE=2.1
     -- Sulky 1147 "Christmas Red"       [swatch] dE=3.4
  ```
- **Click-to-search:** Clicking a thread code triggers search for files using that color

### Step 8: Settings integration

Add thread brand preference to SettingsDialog (Files tab):
- Multi-select checkbox list of available brands
- Stored as `thread_brands` setting (comma-separated)
- Default: "Madeira,Isacord,Brother"

### Step 9: CSS for expanded swatches

New classes in `components.css`:
- `.metadata-swatch-expandable` -- cursor pointer, hover effect
- `.metadata-swatch-matches` -- nested match list container
- `.metadata-swatch-match` -- individual match row
- `.metadata-swatch-delta` -- delta-E display (muted, small)
- `.metadata-swatch-match-code` -- clickable thread code

### Potential Challenges

1. **Thread color data accuracy:** RGB values vary between sources. Use most authoritative sources; architecture allows easy updates.
2. **DST files:** No color data from parsing. Thread mapping only applies to PES, JEF, VP3 files.
3. **UI real estate:** MetadataPanel is a scrollable side panel. Expand/collapse keeps default view compact.
4. **Binary size:** ~100-200KB for static data -- negligible for desktop app.
