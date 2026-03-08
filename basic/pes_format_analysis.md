# PES/PEC Binary Format Analysis

Reverse-engineered from 13 Brother embroidery files (`.PES`, version 6.0).
All files in this analysis were created by Janome software and follow the PES v6 specification.

---

## 1. Overall File Structure

A PES file consists of two main sections:

```
+---------------------------+
| PES Header Section        |  (variable size)
|  - Magic + Version        |
|  - PEC Offset pointer     |
|  - Design metadata        |
|  - Color table (PES)      |
|  - CEmbOne object         |
|  - CSewSeg stitch vectors |
+---------------------------+
| PEC Section               |  (starts at PEC offset)
|  - PEC Label              |
|  - Color palette indices  |
|  - Graphics header        |
|  - Stitch data (compact)  |
|  - Thumbnail images       |
|  - Mini logo bitmaps      |
|  - RGB color values       |
+---------------------------+
```

The PES header provides rich metadata (color names, brands, RGB values, affine transforms). The PEC section provides compact stitch data, palette-indexed colors, and embedded thumbnail images. The PEC section is self-contained and is the same format used in standalone `.PEC` files.

---

## 2. PES Header Structure

### 2.1 File Header (Bytes 0-11)

| Offset | Size | Type      | Description                        | Example                |
|--------|------|-----------|------------------------------------|------------------------|
| 0      | 4    | ASCII     | Magic bytes                        | `#PES`                 |
| 4      | 4    | ASCII     | Version string                     | `0060` (= version 6.0) |
| 8      | 4    | uint32 LE | Absolute byte offset to PEC section | varies per file        |

All examined files use version `0060` (PES v6).

### 2.2 Design Metadata (Bytes 12+)

| Offset | Size | Type      | Description              | Example           |
|--------|------|-----------|--------------------------|--------------------|
| 12     | 2    | uint16 LE | Scale mode / units       | `0x0001`           |
| 14     | 2    | ASCII     | Format indicator         | `"02"`             |
| 16     | 1    | uint8     | Design name length (N)   | 18                 |
| 17     | N    | ASCII     | Design name              | `"BayrischesHerz.JAN"` |

### 2.3 Hoop and Layout Parameters

Starting at offset `17 + name_length + 8` (8 zero padding bytes after name):

| Rel. Offset | Size | Type      | Description                     | Typical Value |
|-------------|------|-----------|---------------------------------|---------------|
| +0          | 2    | uint16 LE | Hoop inner width (mm)           | 126 or 140    |
| +2          | 2    | uint16 LE | Hoop inner height (mm)          | 110 or 200    |
| +4          | 2    | uint16 LE | Unknown (always 0)              | 0             |
| +6          | 2    | uint16 LE | Hoop outer width (mm)           | 200           |
| +8          | 2    | uint16 LE | Hoop outer height (mm)          | 200           |
| +10         | 6    | 3x uint16 | Scaling percentages (100=normal) | 100, 100, 100 |
| +16         | 2    | uint16 LE | Hoop type code                  | 7             |
| +18         | 2    | uint16 LE | Unknown flag                    | 19            |
| +20         | 4    | 2x uint16 | Object counts                   | 1, 1          |
| +24         | 6    | 3x uint16 | Unknown                         | 0, 100, 1     |
| +30         | 4    | zeros     | Padding                         | `00 00 00 00` |
| +34         | 16   | 4x float  | 2x2 Affine transform matrix     | Identity (1,0,0,1) |
| +50         | 13   | zeros     | Padding / translation            | all zeros     |
| +63         | 2    | uint16 LE | **Number of colors**             | 1-11          |

The affine matrix is stored as four IEEE 754 single-precision floats in row-major order: `[a11, a12, a21, a22]`. In all examined files, this is the identity matrix `[1.0, 0.0, 0.0, 1.0]`.

### 2.4 PES Color Objects

Immediately after the color count, each color is encoded as:

```
For each color (repeated num_colors times):
  [1 byte]    Code string length (L1)
  [L1 bytes]  Thread catalog code (ASCII, e.g., "001", "225", "265")
  [3 bytes]   RGB color value (red, green, blue)
  [1 byte]    Separator (0x00)
  [1 byte]    Type flag (always 0x0A = 10)
  [3 bytes]   Padding (0x00 0x00 0x00)
  [1 byte]    Color name length (L2)
  [L2 bytes]  Color name (ASCII, e.g., "White", "Ocean Blue", "Crimson")
  [1 byte]    Brand name length (L3)
  [L3 bytes]  Brand name (ASCII, e.g., "Janome", "Janome Polyester")
  [1 byte]    Separator (0x00)
```

**Verified color objects from examined files:**

| Code | RGB            | Name           | Brand             |
|------|----------------|----------------|--------------------|
| 001  | (255,255,255)  | White          | Janome             |
| 002  | (0,0,0)        | Black          | Janome Polyester   |
| 202  | (240,51,31)    | Vermilion      | Janome             |
| 204  | (255,255,23)   | Yellow         | Janome             |
| 206  | (26,132,45)    | Bright Green   | Janome             |
| 207  | (11,47,132)    | Blue           | Janome             |
| 208  | (171,90,150)   | Purple         | Janome             |
| 209  | (172,156,199)  | Pale Violet    | Janome Polyester   |
| 210  | (252,242,148)  | Pale Yellow    | Janome             |
| 211  | (249,153,183)  | Pale Pink      | Janome             |
| 218  | (127,194,28)   | Yellow Green   | Janome             |
| 222  | (56,108,174)   | Ocean Blue     | Janome             |
| 225  | (255,0,0)      | Red            | Janome             |
| 228  | (178,225,227)  | Baby Blue      | Janome Polyester   |
| 234  | (249,103,107)  | Coral          | Janome Polyester   |
| 250  | (76,191,143)   | Emerald Green  | Janome             |
| 265  | (243,54,137)   | Crimson        | Janome             |

### 2.5 CEmbOne and CSewSeg Sections

After the color objects, the PES header contains embedded objects:

**Terminator pattern** between colors and objects:
```
01 00 FF FF 00 00 07 00
```

**CEmbOne** (Embroidery Object):

| Offset | Size | Description                         |
|--------|------|-------------------------------------|
| 0      | 2    | String length (7 for "CEmbOne")     |
| 2      | 7    | `"CEmbOne"` ASCII                   |
| 9      | 8    | Bounding box: left, top, right, bottom (4x int16 LE, units: 0.1mm) |
| 17     | 8    | Bounding box repeated               |
| 25     | 16   | 2x2 affine transform (4x float32 LE)|
| 41     | 8    | Translation: X, Y (2x float32 LE)   |

The translation floats represent the design center coordinates in 0.1mm. Typically `(1000.0, 1000.0)` for a 200x200mm hoop (center at 100mm from each edge, expressed in 0.1mm).

The bounding box values are absolute coordinates in the hoop coordinate space. For example:
- BayrischesHerz: left=697, top=749, right=1303, bottom=1251
- Width = 1303 - 697 = 606 (0.1mm) = 60.6mm
- Height = 1251 - 749 = 502 (0.1mm) = 50.2mm

**CSewSeg** (Sew Segment): Contains the original stitch path vectors used by the PES editor. This section uses the same string-length-prefix format and contains detailed path data that is not covered in this analysis (the PEC stitch data is the primary source for stitch coordinates).

---

## 3. PEC Section Structure

The PEC section begins at the absolute file offset stored at bytes 8-11 of the PES header.

### 3.1 PEC Label (Bytes 0-18)

| Offset | Size | Description                                    |
|--------|------|------------------------------------------------|
| 0      | 3    | Magic: `"LA:"` (Label prefix)                  |
| 3      | 16   | Design name, right-padded with spaces (0x20)    |

The label is truncated to fit 16 characters. For example, `"BayrischesHerz.JAN"` becomes `"Bayrisch        "`.

### 3.2 PEC Header (Bytes 19-511)

| Offset | Size | Description                                 | Value      |
|--------|------|---------------------------------------------|------------|
| 19     | 1    | Carriage return                             | `0x0D`     |
| 20     | 12   | Padding                                     | 12x `0x20` |
| 32     | 1    | Unknown constant                            | `0xFF`     |
| 33     | 1    | Unknown constant                            | `0x00`     |
| 34     | 1    | Thumbnail width in bytes (pixels / 8)       | `0x06` (48px) |
| 35     | 1    | Thumbnail height in rows                    | `0x26` (38 rows) |
| 36     | 12   | Padding                                     | 12x `0x20` |
| **48** | **1**| **Number of colors minus 1**                | 0-N        |
| 49     | N+1  | PEC palette indices (1 byte each)           | varies     |
| 49+N+1 | ...  | Padding with spaces (0x20) to fill 512 bytes | `0x20`     |

The PEC palette indices reference a fixed 64-color palette. Each index maps to a specific thread color. The mapping from PEC index to RGB observed in these files:

| PEC Index | Color Name     | Approx. RGB         |
|-----------|----------------|----------------------|
| 1         | Blue           | (11, 47, 132)        |
| 4         | Ocean Blue     | (56, 108, 174)       |
| 5         | Red            | (255, 0, 0)          |
| 9         | Purple         | (171, 90, 150)       |
| 13        | Yellow         | (255, 255, 23)       |
| 14        | Yellow Green   | (127, 194, 28)       |
| 20        | Black          | (0, 0, 0)            |
| 25        | Coral          | (249, 103, 107)      |
| 28        | Vermilion      | (240, 51, 31)        |
| 29        | White          | (255, 255, 255)      |
| 31        | Pale Aqua      | (152, 214, 189)      |
| 34        | Pale Yellow    | (252, 242, 148)      |
| 37        | Emerald Green  | (76, 191, 143)       |
| 43        | Pale Pink      | (249, 153, 183)      |
| 45        | Pale Violet    | (172, 156, 199)      |
| 53        | Baby Blue      | (178, 225, 227)      |
| 56        | Bright Green   | (26, 132, 45)        |

### 3.3 Graphics Header (PEC + 512, 20 bytes)

| Offset | Size | Type        | Description                               |
|--------|------|-------------|-------------------------------------------|
| 0      | 2    | uint16 LE   | Unknown (always 0)                        |
| 2      | 3    | uint24 LE   | **Stitch data length** in bytes           |
| 5      | 1    | uint8       | Unknown constant (`0x31`)                 |
| 6      | 2    | uint16 LE   | Unknown constant (`0xFFF0`)               |
| 8      | 2    | uint16 LE   | **Design width** in 0.1mm units           |
| 10     | 2    | uint16 LE   | **Design height** in 0.1mm units          |
| 12     | 2    | uint16 LE   | Hoop display width (always 480)           |
| 14     | 2    | uint16 LE   | Hoop display height (always 432)          |
| 16     | 2    | custom      | **X origin offset** (see encoding below)  |
| 18     | 2    | custom      | **Y origin offset** (see encoding below)  |

**Origin offset encoding**: The X and Y origin offsets store the absolute distance from the origin to the minimum coordinate of the design (i.e., `abs(min_x)` and `abs(min_y)` of the bounding box). The encoding is:

```
value = (high_byte - 0x90) * 256 + low_byte
```

This was verified against all 13 files. For example, BayrischesHerz has bytes `91 2D 90 F8`:
- X offset: `(0x91 - 0x90) * 256 + 0x2D = 301` (matches computed `abs(min_x) = 301`)
- Y offset: `(0x90 - 0x90) * 256 + 0xF8 = 248` (matches computed `abs(min_y) = 248`)

**Design dimensions verified against computed bounds from all 13 files:**

| File              | Header W x H     | Computed W x H    | Match |
|-------------------|-------------------|-------------------|-------|
| BayrischesHerz    | 607 x 503         | 607 x 503         | yes   |
| Blaetter_Puschen  | (verified)        | (verified)        | yes   |
| Bodo              | (verified)        | (verified)        | yes   |
| Boot              | 500 x 500         | 500 x 500         | yes   |
| BrezelHerzen      | (verified)        | (verified)        | yes   |
| Diamant           | (verified)        | (verified)        | yes   |
| Diamant_S         | (verified)        | (verified)        | yes   |
| Diamanten         | (verified)        | (verified)        | yes   |
| Donut             | 504 x 500         | 504 x 500         | yes   |
| Edelweiss         | 391 x 299         | 391 x 299         | yes   |
| Einhorn           | 510 x 440         | 510 x 440         | yes   |
| Erdbeere          | 401 x 449         | 401 x 449         | yes   |
| FAppliHerz        | 572 x 503         | 572 x 503         | yes   |

---

## 4. Stitch Data Encoding

Stitch data begins at PEC offset + 532 (i.e., after the 512-byte PEC header + 20-byte graphics header). The data is a variable-length stream encoding relative displacements.

### 4.1 Encoding Rules

Each stitch is encoded as an X displacement followed by a Y displacement. Three types of encodings exist:

#### Short Form (1 byte per axis)

If the byte has bit 7 clear (value `0x00` to `0x7F`):

```
if byte < 0x40:
    displacement = byte                    (range: 0 to +63)
else:
    displacement = byte - 0x80             (range: -64 to -1)
```

This is 7-bit two's complement: `0x00`=0, `0x01`=+1, ..., `0x3F`=+63, `0x40`=-64, `0x41`=-63, ..., `0x7F`=-1.

A short-form stitch consumes exactly 2 bytes (1 for X, 1 for Y).

#### Long Form (2 bytes per axis)

If the byte has bit 7 set (`0x80` to `0xFE`, excluding special commands):

```
high_byte:
  bit 7: always 1 (long form marker)
  bit 6: unused
  bit 5: jump/trim flag (1 = move without stitching)
  bit 4: unused
  bits 3-0: high 4 bits of displacement

low_byte:
  bits 7-0: low 8 bits of displacement

displacement = (high_byte & 0x0F) << 8 | low_byte    (12-bit unsigned: 0 to 4095)
if displacement >= 0x800:
    displacement -= 0x1000                             (sign extend to: -2048 to +2047)
```

A long-form axis consumes 2 bytes. A stitch can mix short and long forms (e.g., long X + short Y = 3 bytes, or long X + long Y = 4 bytes).

**Jump/Trim flag**: When bit 5 (`0x20`) is set in either the X or Y high byte, the stitch is a positioning move (jump) rather than an actual needle penetration. Jumps are used for:
- Initial positioning at design start
- Moving between disconnected design elements
- Post-color-change repositioning

### 4.2 Special Commands

| Byte Sequence   | Length | Description                                |
|-----------------|--------|--------------------------------------------|
| `0xFE 0xB0 XX`  | 3      | **Color change** (XX = padding/ignored)    |
| `0xFF`           | 1      | **End of stitch data**                     |

**Color change**: The 3-byte sequence `0xFE 0xB0 XX` signals a thread color change. The third byte (`XX`) is consumed but its value is not meaningful for decoding (observed values alternate between `0x01` and `0x02`). After a color change, the next color from the PEC palette index list is used. The number of color changes in the stitch data always equals `num_colors - 1`.

**End marker**: The byte `0xFF` terminates the stitch data stream. It is typically followed by padding zeros up to the stitch data length specified in the graphics header.

### 4.3 Coordinate System

- **Units**: Each displacement unit = 0.1mm
- **Origin**: The first stitch typically starts with a jump to position the needle relative to an arbitrary origin (usually near the design center)
- **Direction**: Positive X = right, positive Y = down (screen coordinates)
- All coordinates are **relative** (delta from previous position)

### 4.4 Verified Stitch Statistics

| File              | Stitches | Jumps | Color Changes | Colors |
|-------------------|----------|-------|---------------|--------|
| BayrischesHerz    | 3,812    | 13    | 1             | 2      |
| Blaetter_Puschen  | 3,122    | 4     | 3             | 4      |
| Bodo              | 3,019    | 15    | 4             | 5      |
| Boot              | 2,498    | 5     | 5             | 6      |
| BrezelHerzen      | 2,623    | 11    | 2             | 3      |
| Diamant           | 2,257    | 6     | 3             | 4      |
| Diamant_S         | 785      | 2     | 3             | 4      |
| Diamanten         | 1,876    | 14    | 3             | 4      |
| Donut             | 4,207    | 17    | 4             | 5      |
| Edelweiss         | 1,538    | 14    | 1             | 2      |
| Einhorn           | 2,447    | 23    | 10            | 11     |
| Erdbeere          | 2,381    | 12    | 4             | 5      |
| FAppliHerz        | 289      | 1     | 0             | 1      |

---

## 5. Thumbnail / Preview Images

After the stitch data, the file contains embedded monochrome thumbnail images.

### 5.1 Thumbnail Dimensions

- **Width**: 48 pixels (6 bytes per row, 1 bit per pixel)
- **Height**: 38 rows
- **Bit depth**: 1 bit per pixel (monochrome)
- **Size**: 6 x 38 = **228 bytes** per image
- **Encoding**: Raw bitmap, no compression. Each byte contains 8 pixels, MSB first.

These dimensions are confirmed by the PEC header at offset +34 (`0x06` = 6 bytes wide) and offset +35 (`0x26` = 38 decimal = 38 rows high).

### 5.2 Number of Thumbnails

There are `num_colors + 1` thumbnail images:

1. **Overview thumbnail** (image 0): Shows all stitches combined as a single monochrome image
2. **Per-color thumbnails** (images 1 through num_colors): Each shows only the stitches for that color. The per-color thumbnails often contain the PEC palette index embedded in the bottom rows of the image (replacing the last 3-4 rows of pixel data with the index value and space padding `0x20`)

### 5.3 Thumbnail Location

```
thumbnail_start = PEC_offset + 532 + stitch_data_length
image_N_offset  = thumbnail_start + N * 228     (N = 0, 1, ..., num_colors)
```

### 5.4 Thumbnail Image Format

Each row is 6 bytes = 48 pixels. Pixels are packed MSB-first within each byte:

```
Byte:    b7 b6 b5 b4 b3 b2 b1 b0
Pixels:  p0 p1 p2 p3 p4 p5 p6 p7
```

A set bit (1) represents a filled/stitched pixel; a clear bit (0) represents background.

The thumbnail images include a decorative border frame around the design preview. The border consists of a thin rectangular outline near the edges of the 48x38 pixel area.

### 5.5 How to Extract a Thumbnail

To extract the overview thumbnail as a raw bitmap:

1. Seek to `PEC_offset + 532 + stitch_data_length`
2. Read 228 bytes (6 bytes x 38 rows)
3. Each row is 6 bytes = 48 pixels, MSB-first
4. Render as monochrome: bit=1 is foreground (design), bit=0 is background

---

## 6. Post-Thumbnail Metadata

After the `(num_colors + 1) * 228` bytes of thumbnails, a metadata tail follows until the end of file.

### 6.1 Structure

```
[spaces padding]                          variable length (0x20 bytes)
[mini logo bitmap + padding] x num_colors  144 bytes per color
[zeros padding]                            variable length
[RGB color values]                         num_colors * 3 bytes
[end padding]                              2 bytes (0x00 0x00)
```

### 6.2 Mini Logo Bitmap

Each color's metadata block contains a 42-byte monochrome bitmap (the manufacturer/software logo, e.g., Janome). This bitmap is:
- **Width**: 48 pixels (6 bytes per row)
- **Height**: 7 rows
- **Size**: 6 x 7 = 42 bytes

The logo is identical across all colors and all files (for the same software). The blocks are spaced exactly **144 bytes** apart. The first logo block starts at offset 198 from the end of the thumbnail images.

### 6.3 Trailing RGB Color Values

The last `num_colors * 3 + 2` bytes of the file contain:

```
[R1 G1 B1] [R2 G2 B2] ... [Rn Gn Bn] [0x00 0x00]
```

These RGB values match the color definitions from the PES header color objects. They provide a quick way to look up thread colors without parsing the full PES header.

**Verified**: The RGB values at the end of every examined file match the RGB values from the corresponding PES color objects exactly.

---

## 7. Summary of Key Offsets

### Reading a PES file - Quick Reference

```
1. Read magic at offset 0: "#PES" (4 bytes)
2. Read version at offset 4: "0060" (4 bytes)
3. Read PEC offset at offset 8: uint32 LE

4. Read design name:
   - Name length at offset 16: uint8
   - Name string at offset 17: ASCII

5. Read PES colors:
   - Color count at offset 17 + name_len + 8 + 63: uint16 LE
   - Color objects follow immediately

6. Read PEC section (at PEC offset):
   - Label: offset +0, 19 bytes ("LA:" + 16-char name)
   - Num colors - 1: offset +48, 1 byte
   - Palette indices: offset +49, (num_colors) bytes
   - Stitch data length: offset +514, 3-byte uint24 LE
   - Design width: offset +520, uint16 LE (0.1mm)
   - Design height: offset +522, uint16 LE (0.1mm)
   - Stitch data: offset +532, variable length
   - Thumbnails: offset +532 + stitch_len, (num_colors+1) * 228 bytes

7. End of file:
   - RGB values: last (num_colors * 3 + 2) bytes
```

### Critical Format Details

- **Color change command is 3 bytes** (`0xFE 0xB0 XX`), not 2. The third byte is padding and must be consumed to maintain byte alignment. Failure to consume this byte causes progressive decoder misalignment that corrupts all subsequent stitch data and color change detection.
- **Stitch displacement encoding** uses 7-bit two's complement for short form (range -64 to +63) and 12-bit two's complement for long form (range -2048 to +2047).
- **Thumbnail images** are always 48x38 pixels at 1 bit per pixel, totaling 228 bytes per image.
- **All multi-byte integer values** are little-endian unless otherwise noted.
- **The stitch data length** at PEC+514 is a 24-bit (3-byte) little-endian unsigned integer.

---

## 8. File Size Composition

Example breakdown for `Boot.PES` (18,822 bytes, 6 colors):

| Section                    | Offset  | Size    | Details                        |
|----------------------------|---------|---------|--------------------------------|
| PES header                 | 0       | 10,652  | Metadata, colors, CSewSeg      |
| PEC label + palette header | 10,652  | 512     | Label, palette indices         |
| PEC graphics header        | 11,164  | 20      | Dimensions, stitch length      |
| Stitch data                | 11,184  | 5,050   | 2,498 stitches + 5 CCs        |
| Thumbnail images           | 16,234  | 1,596   | 7 images x 228 bytes           |
| Metadata tail              | 17,830  | 992     | Logos, spaces, RGB, padding    |
| **Total**                  |         | **18,822** |                             |

---

*Analysis performed on 2026-03-08 by examining 13 PES files with xxd/hexdump and verifying decoded values against computed stitch bounds. All format details were cross-validated against multiple files.*
