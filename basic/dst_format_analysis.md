# DST (Tajima Embroidery) Binary Format Analysis

Analysis date: 2026-03-08

Based on reverse-engineering of 4 sample DST files:

| File | Size (bytes) | Stitches (ST) | Colors (CO) | Dimensions (mm) |
|------|-------------|---------------|-------------|-----------------|
| 2.DST | 82,278 | 27,255 | 5 | 101.8 x 127.2 |
| 3 Ohren L.DST | 5,553 | 1,680 | 1 | 100.8 x 148.9 |
| 4.DST | 68,820 | 22,769 | 0 | 124.5 x 127.3 |
| 5X7_Follow...Fill.dst | 45,348 | 14,904 | 4 | 127.4 x 124.8 |

---

## 1. Overall File Structure

A DST file consists of three parts laid out sequentially:

```
[ 512-byte Header ] [ Stitch Data (3-byte triplets) ] [ 0x1A ]
```

- **Header**: Always exactly 512 bytes. Contains ASCII metadata fields followed by 0x20 (space) padding.
- **Stitch Data**: Variable length. Each command is a 3-byte triplet encoding movement and flags.
- **Trailing Byte**: The file always ends with a single `0x1A` byte (ASCII SUB / EOF marker).

The stitch data itself ends with a special END triplet `00 00 F3` followed by the `0x1A` byte.

---

## 2. Header Structure (512 bytes)

The header consists of labeled ASCII fields separated by `0x0D` (carriage return), terminated by `0x1A`, then padded with `0x20` (space) to fill 512 bytes.

### Field Layout

All four sample files use identical field positions:

| Offset (hex) | Offset (dec) | Label | Width | Description |
|--------------|-------------|-------|-------|-------------|
| 0x000 | 0 | `LA:` | 19 chars + CR | Design label/name (space-padded to 16 chars after `LA:`) |
| 0x014 | 20 | `ST:` | 7 chars + CR | Stitch count (total triplets including END) |
| 0x01F | 31 | `CO:` | 3 chars + CR | Color change count |
| 0x026 | 38 | `+X:` | 5 chars + CR | Maximum positive X extent from origin (in 0.1mm units) |
| 0x02F | 47 | `-X:` | 5 chars + CR | Maximum negative X extent from origin (in 0.1mm units) |
| 0x038 | 56 | `+Y:` | 5 chars + CR | Maximum positive Y extent from origin (in 0.1mm units) |
| 0x041 | 65 | `-Y:` | 5 chars + CR | Maximum negative Y extent from origin (in 0.1mm units) |
| 0x04A | 74 | `AX:` | 6 chars + CR | End position X offset from start (signed, with `+`/`-` prefix) |
| 0x054 | 84 | `AY:` | 6 chars + CR | End position Y offset from start (signed, with `+`/`-` prefix) |
| 0x05E | 94 | `MX:` | 6 chars + CR | Unknown (always `+    0` in samples) |
| 0x068 | 104 | `MY:` | 6 chars + CR | Unknown (always `+    0` in samples) |
| 0x072 | 114 | `PD:` | 6 chars + CR | Unknown/previous design pointer (always `******` in samples) |
| 0x07C | 124 | | 1 byte | `0x1A` header terminator |
| 0x07D | 125 | | 387 bytes | Padding with `0x20` (space) to fill 512 bytes |

### Field Details

**LA (Label)**: The design name, truncated to fit the 16-character field. Space-padded on the right. Example: `LA:3 Ohren         ` or `LA:5X7_Foll        `. The label is always followed by a `0x0D` carriage return.

**ST (Stitch Count)**: Total number of 3-byte triplets in the stitch data, **including the END marker triplet**. Right-justified, space-padded. Verified against 3 of 4 files:

| File | Header ST | Actual Triplets (incl. END) | Match |
|------|-----------|---------------------------|-------|
| 2.DST | 27,255 | 27,255 | Exact |
| 3 Ohren L.DST | 1,680 | 1,680 | Exact |
| 4.DST | 22,769 | 22,769 | Exact |
| 5X7_Follow...dst | 14,904 | 14,945 | No (see note) |

Note: The bunny file discrepancy (41 fewer in header) suggests different embroidery software may count ST differently. The other 3 files count ST as the total number of triplets including the END marker.

**CO (Color Changes)**: Number of color change commands in the stitch data. A design with CO=0 uses a single thread color. CO=5 means 5 color changes, requiring 6 distinct thread colors. Verified to match the actual count of `0xC3` triplets in 3 of 4 files. The bunny file has CO=4 in the header but only 3 color change commands in the data.

**+X, -X, +Y, -Y (Extents)**: Bounding box extents from the design origin (starting needle position), measured in DST units (1 unit = 0.1 mm). These represent the maximum positive and negative displacement reached during stitching. To compute design dimensions:

```
Width  (mm) = (+X + -X) * 0.1
Height (mm) = (+Y + -Y) * 0.1
```

Verified against all 4 files by accumulating stitch displacements. Results match within 0-3 units (minor rounding differences).

**AX, AY (End Position)**: The cumulative X and Y displacement from start to the final stitch position, in DST units. Includes a sign character (`+` or `-`). Verified to match exactly by summing all dx/dy values from stitch data in all 4 files.

**MX, MY**: Purpose unknown. Always `+    0` in all samples. Possibly reserved for multi-head machine offsets.

**PD**: Purpose unknown. Always `******` in all samples. Possibly a pointer field for linking designs.

---

## 3. Stitch Data Encoding (3-Byte Triplets)

Stitch data begins immediately after the 512-byte header (at file offset 0x200). Each command is exactly 3 bytes, encoding:
- An X displacement (dx) in the range -121 to +121
- A Y displacement (dy) in the range -121 to +121
- Command flags (normal stitch, jump, color change, or end)

### First Stitch Convention

All four sample files begin with the triplet `00 00 83`, which is a jump stitch with zero displacement. This serves as an initializer before the actual stitch path begins.

### Bit Layout

The displacement values are encoded using **balanced ternary** across the 3 bytes. Each axis uses 5 weight levels (1, 3, 9, 27, 81) with two bits per level (one for positive, one for negative). The two bits in a pair are never simultaneously set.

#### Byte 0 (first byte read)

| Bit | Weight | Axis |
|-----|--------|------|
| 7 | +1 | Y |
| 6 | -1 | Y |
| 5 | +9 | Y |
| 4 | -9 | Y |
| 3 | -9 | X |
| 2 | +9 | X |
| 1 | -1 | X |
| 0 | +1 | X |

#### Byte 1 (second byte read)

| Bit | Weight | Axis |
|-----|--------|------|
| 7 | +3 | Y |
| 6 | -3 | Y |
| 5 | +27 | Y |
| 4 | -27 | Y |
| 3 | -27 | X |
| 2 | +27 | X |
| 1 | -3 | X |
| 0 | +3 | X |

#### Byte 2 (third byte read)

| Bit | Weight | Axis |
|-----|--------|------|
| 7 | JUMP flag | Control |
| 6 | COLOR CHANGE flag | Control |
| 5 | +81 | Y |
| 4 | -81 | Y |
| 3 | -81 | X |
| 2 | +81 | X |
| 1 | Always 1 | Fixed |
| 0 | Always 1 | Fixed |

### Decode Algorithm

```
dx = bit(b2,2)*81 - bit(b2,3)*81
   + bit(b1,2)*27 - bit(b1,3)*27
   + bit(b0,2)*9  - bit(b0,3)*9
   + bit(b1,0)*3  - bit(b1,1)*3
   + bit(b0,0)*1  - bit(b0,1)*1

dy = bit(b2,5)*81 - bit(b2,4)*81
   + bit(b1,5)*27 - bit(b1,4)*27
   + bit(b0,5)*9  - bit(b0,4)*9
   + bit(b1,7)*3  - bit(b1,6)*3
   + bit(b0,7)*1  - bit(b0,6)*1
```

where `bit(byte, pos) = (byte >> pos) & 1`.

**Coordinate system note**: In the raw DST format, positive X is to the right, and positive Y values as decoded above correspond to the `+Y` extent in the header. Some software (e.g., pyembroidery) negates dy for screen display (Y-down vs Y-up convention), but the raw encoding matches the header extents without negation.

### Displacement Range

Each axis: -121 to +121 DST units (= -12.1 to +12.1 mm per stitch). This is the maximum single-stitch displacement, corresponding to the maximum balanced ternary value: -(1+3+9+27+81) to +(1+3+9+27+81).

### Fixed Bits

Bits 0 and 1 of byte 2 are **always set** (value 0x03) across all stitch commands in all sample files. This means byte 2 always has a minimum value of 0x03 for normal stitches. These bits serve as a format marker.

---

## 4. Command Types (Byte 2 Flags)

The command type is determined by bits 7 and 6 of byte 2:

| Bit 7 | Bit 6 | Byte 2 base | Type | Description |
|-------|-------|-------------|------|-------------|
| 0 | 0 | 0x03 | Normal Stitch | Needle penetrates fabric; thread is laid |
| 1 | 0 | 0x83 | Jump Stitch | Needle moves without penetrating; thread is carried above fabric |
| 1 | 1 | 0xC3 | Color Change | Machine stops for thread change; always `00 00 C3` (zero displacement) |
| - | - | 0xF3 | End of Data | Special: `00 00 F3`; terminates stitch data |

### Observed byte 2 value distribution (2.DST, 27,254 commands):

| Value | Binary | Count | Meaning |
|-------|--------|-------|---------|
| 0x03 | 00000011 | 27,096 | Normal stitch, no large X/Y |
| 0x07 | 00000111 | 7 | Normal stitch, X includes +81 component |
| 0x0B | 00001011 | 6 | Normal stitch, X includes -81 component |
| 0x13 | 00010011 | 5 | Normal stitch, Y includes -81 component |
| 0x23 | 00100011 | 2 | Normal stitch, Y includes +81 component |
| 0x83 | 10000011 | 46 | Jump stitch, no large X/Y |
| 0x87 | 10000111 | 17 | Jump stitch, X includes +81 |
| 0x8B | 10001011 | 20 | Jump stitch, X includes -81 |
| 0x93 | 10010011 | 22 | Jump stitch, Y includes -81 |
| 0xA3 | 10100011 | 9 | Jump stitch, Y includes +81 |
| 0xC3 | 11000011 | 5 | Color change |

---

## 5. Color Changes

Color change commands are always encoded as the exact triplet `00 00 C3`:
- Byte 0 = 0x00, Byte 1 = 0x00, Byte 2 = 0xC3
- Zero displacement (dx=0, dy=0)
- Both JUMP and COLOR CHANGE flags are set

### Color Change Pattern

A color change always follows this sequence:
1. Final normal stitches of the current color
2. `00 00 C3` -- color change command
3. Multiple jump stitches (typically 3-13) moving to the start of the next color region
4. Normal stitches resume in the new color

Example from 2.DST (first color change at triplet #17,353):
```
[17350] 46 40 03  dx=+8  dy=-4   (normal stitch)
[17351] 09 80 03  dx=-8  dy=+3   (normal stitch)
[17352] 84 40 03  dx=+9  dy=-2   (normal stitch)
[17353] 00 00 C3  dx=+0  dy=+0   (COLOR CHANGE)
[17354] 55 02 8B  dx=-74 dy=-10  (jump to next region)
[17355] 15 01 8B  dx=-68 dy=-9   (jump continues)
[17356] 15 01 8B  dx=-68 dy=-9   (jump continues)
```

### Color Count (CO) Semantics

The CO header field counts the number of color **changes** (stop commands), not the number of distinct colors. A design with CO=0 uses 1 color. A design with CO=5 uses 6 colors. The number of thread colors needed is `CO + 1` (when CO > 0) or `1` (when CO = 0).

**Important**: DST files do NOT store actual color values (RGB, palette index, etc.). The CO field only tells the machine how many times to stop for a thread change. Color assignment is handled externally by the operator or by a separate color sequence file.

---

## 6. Jump Stitches and Trim Sequences

Jump stitches (byte 2 bit 7 set) move the needle without penetrating the fabric. They are used for:
1. **Initial positioning**: Moving from the origin to the first stitch location
2. **Color change transitions**: Moving between color regions after a color change
3. **Within-design jumps**: Moving between disconnected areas of the same color

### Trim Convention

DST has no explicit "trim" command. Instead, a **sequence of consecutive jump stitches** signals a trim (thread cut). Embroidery machines interpret 2 or more consecutive jumps as a trim followed by repositioning. In the sample files, jump sequences range from 2 to 13 consecutive jumps.

Jump stitch sequence length distribution (bunny file):
- Length 3: 13 sequences
- Length 4: 4 sequences
- Length 13: 2 sequences
- Other lengths (2, 7, 8, 9, 11): 1 each

---

## 7. End of Data Marker

The stitch data terminates with the triplet `00 00 F3`:

```
Byte 2 = 0xF3 = 11110011:
  bit 7 = 1 (JUMP)
  bit 6 = 1 (COLOR CHANGE)
  bit 5 = 1 (Y +81)
  bit 4 = 1 (Y -81)
  bits 1,0 = 11 (fixed)
```

This is a special combination where both Y +81 and Y -81 are set simultaneously (which never occurs in normal stitches since the bits are mutually exclusive), combined with both JUMP and COLOR CHANGE flags. This makes `F3` an unmistakable sentinel.

After the END triplet, the file contains exactly one `0x1A` byte and then the file ends. This was verified in all 4 sample files.

### File Size Formula

```
file_size = 512 (header) + ST * 3 (stitch triplets including END) + 1 (trailing 0x1A)
```

Verified for 3 of 4 files. The bunny file deviates due to its non-standard ST count.

---

## 8. Dimensions and Units

### DST Unit System

1 DST unit = 0.1 mm = 1/254 inch (approximately)

### Computing Design Dimensions from Header

```
Width  = (+X + -X) * 0.1 mm
Height = (+Y + -Y) * 0.1 mm
```

The `+X` and `-X` values represent the maximum distance the needle travels in the positive and negative X direction from the starting point (origin). They are unsigned values despite representing extent in a signed direction.

### Verified Dimensions

| File | +X | -X | +Y | -Y | Width (mm) | Height (mm) |
|------|----|----|----|----|-----------|-------------|
| 2.DST | 509 | 509 | 636 | 636 | 101.8 | 127.2 |
| 3 Ohren L.DST | 440 | 568 | 1360 | 129 | 100.8 | 148.9 |
| 4.DST | 622 | 623 | 637 | 636 | 124.5 | 127.3 |
| 5X7_Follow...dst | 637 | 637 | 624 | 624 | 127.4 | 124.8 |

Note: When `+X` equals `-X` (and `+Y` equals `-Y`), the design is centered on its starting point. Asymmetric values (like 3 Ohren L.DST: +Y=1360 vs -Y=129) indicate the starting point is not at the design center.

---

## 9. Design Label Storage

The label is stored in the first 20 bytes of the file:
- Bytes 0-2: `LA:` (ASCII literal)
- Bytes 3-18: Design name, 16 characters max, right-padded with spaces (0x20)
- Byte 19: `0x0D` (carriage return delimiter)

The label is always exactly 16 characters (padded). If the design name is longer than 16 characters, it is truncated. Examples:

| File | Full Filename | LA Value (16 chars) |
|------|--------------|---------------------|
| 2.DST | 2.DST | `2               ` |
| 3 Ohren L.DST | 3 Ohren L.DST | `3 Ohren         ` |
| 4.DST | 4.DST | `4               ` |
| 5X7_Follow...dst | 5X7_FollowTheBunny... | `5X7_Foll        ` |

Note: The label for the bunny file is truncated to `5X7_Foll` -- only 8 characters of the original name are preserved. The truncation point varies; the format only guarantees 16 characters maximum.

---

## 10. Summary of Key Format Properties

| Property | Value |
|----------|-------|
| Header size | Fixed 512 bytes |
| Stitch command size | Fixed 3 bytes |
| Max displacement per stitch | +/-121 units (+/-12.1 mm) |
| Unit resolution | 0.1 mm |
| Coordinate system | Relative (delta from previous position) |
| Color information | None (only change count stored) |
| End marker | `00 00 F3` followed by `0x1A` |
| First stitch | Always `00 00 83` (zero-displacement jump) |
| Header terminator | `0x1A` at offset 124 |
| Header padding | `0x20` from offset 125 to 511 |
| Balanced ternary weights | 1, 3, 9, 27, 81 |
| Byte 2 fixed bits | Bits 0 and 1 always set (0x03 minimum) |

### Byte 2 Quick Reference

| Value | Meaning |
|-------|---------|
| 0x03 | Normal stitch (no +/-81 components) |
| 0x83 | Jump stitch (no +/-81 components) |
| 0xC3 | Color change (always with `00 00` for zero displacement) |
| 0xF3 | End of data (always `00 00 F3`) |

### Software Compatibility Notes

The header field values (especially ST and CO) may vary slightly depending on the software that generated the DST file. Three of the four sample files show perfect consistency (ST = total triplets including END, CO = exact count of 0xC3 commands). The fourth file (generated by different software) shows minor discrepancies: ST is 41 less than the actual triplet count, and CO is 1 more than the actual color change command count. Parsers should rely on the actual stitch data rather than header values for precise counts.
