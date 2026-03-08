#!/usr/bin/env python3
"""Extract metadata, thumbnails, and render preview images from PES (Brother) embroidery files.
Based on format analysis in pes_format_analysis.md"""

import struct
import json
import os
import sys
from PIL import Image, ImageDraw

EXAMPLE_DIR = os.path.join(os.path.dirname(__file__), "..", "example files")
OUTPUT_DIR = os.path.dirname(__file__)

# Known PEC palette (index -> RGB), from analysis
PEC_PALETTE = {
    1: (14, 31, 124),    2: (10, 85, 163),    3: (48, 135, 119),
    4: (75, 107, 175),   5: (237, 23, 31),    6: (209, 92, 0),
    7: (145, 54, 151),   8: (228, 154, 203),  9: (145, 95, 172),
    10: (158, 214, 125), 11: (232, 169, 0),   12: (254, 186, 53),
    13: (255, 255, 0),   14: (112, 188, 31),  15: (186, 152, 0),
    16: (168, 168, 168), 17: (123, 111, 0),   18: (255, 0, 0),
    19: (209, 166, 116), 20: (0, 0, 0),       21: (0, 0, 0),
    22: (255, 204, 204), 23: (221, 132, 132), 24: (255, 153, 0),
    25: (255, 102, 102), 26: (0, 51, 0),      27: (0, 102, 0),
    28: (206, 59, 10),   29: (255, 255, 255), 30: (200, 200, 200),
    31: (150, 200, 178), 32: (190, 152, 190), 33: (180, 133, 230),
    34: (255, 240, 141), 35: (226, 200, 168), 36: (255, 190, 190),
    37: (76, 191, 143),  38: (228, 227, 220), 39: (0, 130, 90),
    40: (220, 210, 130), 41: (190, 166, 212), 42: (70, 92, 148),
    43: (250, 150, 180), 44: (180, 132, 190), 45: (180, 160, 200),
    46: (200, 170, 148), 47: (200, 170, 120), 48: (130, 200, 148),
    49: (160, 180, 170), 50: (168, 138, 103), 51: (186, 100, 172),
    52: (0, 128, 192),   53: (175, 210, 220), 54: (0, 192, 0),
    55: (0, 96, 0),      56: (39, 133, 56),   57: (200, 200, 200),
    58: (200, 200, 255), 59: (30, 30, 30),    60: (100, 100, 100),
    61: (200, 200, 200), 62: (180, 0, 0),     63: (0, 0, 128),
    64: (255, 200, 200),
}


def parse_pes_header(data):
    """Parse PES header: magic, version, PEC offset, design name, colors."""
    magic = data[0:4].decode("ascii")
    version = data[4:8].decode("ascii")
    pec_offset = struct.unpack_from("<I", data, 8)[0]

    # Design name
    name_len = data[16]
    design_name = data[17:17 + name_len].decode("ascii", errors="replace")

    # Color count and color objects
    color_offset = 17 + name_len + 8 + 63  # after name + padding + layout params
    num_colors = struct.unpack_from("<H", data, color_offset)[0]

    colors = []
    pos = color_offset + 2
    for _ in range(num_colors):
        code_len = data[pos]
        pos += 1
        code = data[pos:pos + code_len].decode("ascii", errors="replace")
        pos += code_len
        r, g, b = data[pos], data[pos + 1], data[pos + 2]
        pos += 3
        pos += 1  # separator 0x00
        pos += 1  # type flag (0x0A)
        pos += 3  # padding
        name_len2 = data[pos]
        pos += 1
        color_name = data[pos:pos + name_len2].decode("ascii", errors="replace")
        pos += name_len2
        brand_len = data[pos]
        pos += 1
        brand = data[pos:pos + brand_len].decode("ascii", errors="replace")
        pos += brand_len
        pos += 1  # separator 0x00

        colors.append({
            "code": code,
            "rgb": [r, g, b],
            "name": color_name,
            "brand": brand,
        })

    return {
        "magic": magic,
        "version": version,
        "pec_offset": pec_offset,
        "design_name": design_name,
        "num_colors": num_colors,
        "colors": colors,
    }


def parse_pec_header(data, pec_offset):
    """Parse PEC section header."""
    label = data[pec_offset + 3:pec_offset + 19].decode("ascii", errors="replace").strip()
    num_colors_minus1 = data[pec_offset + 48]
    num_colors = num_colors_minus1 + 1

    palette_indices = []
    for i in range(num_colors):
        palette_indices.append(data[pec_offset + 49 + i])

    # Graphics header at PEC + 512
    gh_offset = pec_offset + 512
    stitch_data_len = struct.unpack_from("<I", data, gh_offset + 2)[0] & 0xFFFFFF  # 24-bit
    width = struct.unpack_from("<H", data, gh_offset + 8)[0]
    height = struct.unpack_from("<H", data, gh_offset + 10)[0]

    return {
        "label": label,
        "num_colors": num_colors,
        "palette_indices": palette_indices,
        "stitch_data_length": stitch_data_len,
        "width_01mm": width,
        "height_01mm": height,
        "stitch_data_offset": pec_offset + 532,
    }


def decode_pec_stitches(data, stitch_offset, stitch_len):
    """Decode PEC stitch data. Returns list of (x, y, cmd, color_index)."""
    stitches = []
    x, y = 0.0, 0.0
    pos = stitch_offset
    end = stitch_offset + stitch_len
    color_index = 0
    normal_count = 0
    jump_count = 0

    while pos < end:
        b = data[pos]

        # End marker
        if b == 0xFF:
            break

        # Color change: 0xFE 0xB0 XX (3 bytes)
        if b == 0xFE and pos + 1 < end and data[pos + 1] == 0xB0:
            color_index += 1
            pos += 3  # consume all 3 bytes
            continue

        # Decode X displacement
        is_jump = False
        if b & 0x80:
            # Long form (2 bytes)
            if pos + 1 >= end:
                break
            high = b
            low = data[pos + 1]
            pos += 2
            if high & 0x20:
                is_jump = True
            dx_raw = ((high & 0x0F) << 8) | low
            if dx_raw >= 0x800:
                dx_raw -= 0x1000
            dx = dx_raw
        else:
            # Short form (1 byte)
            pos += 1
            if b < 0x40:
                dx = b
            else:
                dx = b - 0x80

        # Decode Y displacement
        if pos >= end:
            break
        b = data[pos]

        if b & 0x80:
            # Long form
            if pos + 1 >= end:
                break
            high = b
            low = data[pos + 1]
            pos += 2
            if high & 0x20:
                is_jump = True
            dy_raw = ((high & 0x0F) << 8) | low
            if dy_raw >= 0x800:
                dy_raw -= 0x1000
            dy = dy_raw
        else:
            pos += 1
            if b < 0x40:
                dy = b
            else:
                dy = b - 0x80

        x += dx * 0.1
        y += dy * 0.1

        cmd = "JUMP" if is_jump else "STITCH"
        if cmd == "STITCH":
            normal_count += 1
        else:
            jump_count += 1

        stitches.append((x, y, cmd, color_index))

    return stitches, normal_count, jump_count


def extract_thumbnail(data, pec_offset, stitch_data_len):
    """Extract the overview thumbnail (48x38 monochrome) as a PIL Image."""
    thumb_offset = pec_offset + 532 + stitch_data_len
    img = Image.new("1", (48, 38), 0)
    pixels = img.load()

    for row in range(38):
        for byte_idx in range(6):
            b = data[thumb_offset + row * 6 + byte_idx]
            for bit in range(8):
                if b & (0x80 >> bit):
                    pixels[byte_idx * 8 + bit, row] = 1

    return img


def render_preview(stitches, colors_rgb, filename):
    """Render a full-color stitch preview as PNG."""
    if not stitches:
        return

    stitch_points = [(s[0], s[1]) for s in stitches if s[2] == "STITCH"]
    if not stitch_points:
        return

    xs = [p[0] for p in stitch_points]
    ys = [p[1] for p in stitch_points]
    min_x, max_x = min(xs), max(xs)
    min_y, max_y = min(ys), max(ys)
    design_w = max_x - min_x
    design_h = max_y - min_y

    if design_w == 0 or design_h == 0:
        return

    margin = 40
    img_size = 800
    scale = min((img_size - 2 * margin) / design_w,
                (img_size - 2 * margin) / design_h)

    img = Image.new("RGB", (img_size, img_size), (255, 255, 255))
    draw = ImageDraw.Draw(img)

    prev_x, prev_y = None, None
    for x, y, cmd, ci in stitches:
        sx = margin + (x - min_x) * scale
        sy = margin + (y - min_y) * scale

        if cmd == "STITCH" and prev_x is not None:
            color = tuple(colors_rgb[ci % len(colors_rgb)]) if colors_rgb else (0, 0, 0)
            draw.line([(prev_x, prev_y), (sx, sy)], fill=color, width=1)

        if cmd == "STITCH" or cmd == "JUMP":
            prev_x, prev_y = sx, sy
        else:
            prev_x, prev_y = None, None

    img.save(filename)


def process_pes(filepath):
    """Process a single PES file and return metadata dict."""
    basename = os.path.splitext(os.path.basename(filepath))[0]
    print(f"  Processing: {os.path.basename(filepath)}")

    with open(filepath, "rb") as f:
        data = f.read()

    pes = parse_pes_header(data)
    pec = parse_pec_header(data, pes["pec_offset"])

    stitches, normal_count, jump_count = decode_pec_stitches(
        data, pec["stitch_data_offset"], pec["stitch_data_length"]
    )

    width_mm = pec["width_01mm"] * 0.1
    height_mm = pec["height_01mm"] * 0.1

    # Get RGB colors from PES header color objects
    colors_rgb = [c["rgb"] for c in pes["colors"]]

    # Fallback to PEC palette if PES colors unavailable
    if not colors_rgb:
        colors_rgb = [list(PEC_PALETTE.get(idx, (0, 0, 0))) for idx in pec["palette_indices"]]

    # Render full preview
    preview_name = f"{basename}_preview.png"
    preview_path = os.path.join(OUTPUT_DIR, preview_name)
    render_preview(stitches, colors_rgb, preview_path)

    # Extract embedded thumbnail
    thumb_name = f"{basename}_thumbnail.png"
    thumb_path = os.path.join(OUTPUT_DIR, thumb_name)
    try:
        thumb = extract_thumbnail(data, pes["pec_offset"], pec["stitch_data_length"])
        # Scale up for visibility
        thumb_scaled = thumb.resize((192, 152), Image.NEAREST)
        thumb_scaled.save(thumb_path)
    except Exception as e:
        print(f"    Warning: Could not extract thumbnail: {e}")
        thumb_name = None

    info = {
        "filename": os.path.basename(filepath),
        "format": "PES (Brother)",
        "version": pes["version"],
        "design_name": pes["design_name"],
        "label": pec["label"],
        "stitch_count": normal_count,
        "jump_count": jump_count,
        "num_colors": pes["num_colors"],
        "colors": pes["colors"],
        "pec_palette_indices": pec["palette_indices"],
        "width_mm": round(width_mm, 1),
        "height_mm": round(height_mm, 1),
        "file_size_bytes": len(data),
        "preview_image": preview_name,
        "thumbnail_image": thumb_name,
    }

    # Save individual JSON
    json_path = os.path.join(OUTPUT_DIR, f"{basename}_info.json")
    with open(json_path, "w") as f:
        json.dump(info, f, indent=2, ensure_ascii=False)

    return info


def main():
    print("=== PES File Extraction ===")
    pes_files = sorted([
        os.path.join(EXAMPLE_DIR, f) for f in os.listdir(EXAMPLE_DIR)
        if f.lower().endswith(".pes")
    ])

    if not pes_files:
        print("No PES files found.")
        return

    all_info = []
    for fp in pes_files:
        try:
            info = process_pes(fp)
            all_info.append(info)
            print(f"    Name: {info['design_name']}")
            print(f"    Stitches: {info['stitch_count']}, Jumps: {info['jump_count']}")
            print(f"    Colors: {info['num_colors']} - {', '.join(c['name'] for c in info['colors'])}")
            print(f"    Size: {info['width_mm']} x {info['height_mm']} mm")
            print()
        except Exception as e:
            print(f"    ERROR: {e}")
            import traceback
            traceback.print_exc()
            print()

    # Save combined summary
    summary_path = os.path.join(OUTPUT_DIR, "pes_summary.json")
    with open(summary_path, "w") as f:
        json.dump(all_info, f, indent=2, ensure_ascii=False)
    print(f"Summary saved to: {summary_path}")


if __name__ == "__main__":
    main()
