#!/usr/bin/env python3
"""Extract metadata and render preview images from DST (Tajima) embroidery files.
Based on format analysis in dst_format_analysis.md"""

import struct
import json
import os
import sys
from PIL import Image, ImageDraw

EXAMPLE_DIR = os.path.join(os.path.dirname(__file__), "..", "example files")
OUTPUT_DIR = os.path.dirname(__file__)


def parse_header(data):
    """Parse the 512-byte DST header."""
    header = {}
    header["label"] = data[3:19].decode("ascii", errors="replace").strip()
    header["stitch_count"] = int(data[23:30].decode("ascii").strip())
    header["color_changes"] = int(data[34:37].decode("ascii").strip())
    header["plus_x"] = int(data[41:46].decode("ascii").strip())
    header["minus_x"] = int(data[50:55].decode("ascii").strip())
    header["plus_y"] = int(data[59:64].decode("ascii").strip())
    header["minus_y"] = int(data[68:73].decode("ascii").strip())
    header["ax"] = data[77:83].decode("ascii").strip()
    header["ay"] = data[87:93].decode("ascii").strip()
    return header


def decode_stitch(b0, b1, b2):
    """Decode a 3-byte DST stitch triplet into (dx, dy, command)."""
    bit = lambda byte, pos: (byte >> pos) & 1

    dx = (bit(b2, 2) * 81 - bit(b2, 3) * 81
          + bit(b1, 2) * 27 - bit(b1, 3) * 27
          + bit(b0, 2) * 9 - bit(b0, 3) * 9
          + bit(b1, 0) * 3 - bit(b1, 1) * 3
          + bit(b0, 0) * 1 - bit(b0, 1) * 1)

    dy = (bit(b2, 5) * 81 - bit(b2, 4) * 81
          + bit(b1, 5) * 27 - bit(b1, 4) * 27
          + bit(b0, 5) * 9 - bit(b0, 4) * 9
          + bit(b1, 7) * 3 - bit(b1, 6) * 3
          + bit(b0, 7) * 1 - bit(b0, 6) * 1)

    if b2 == 0xF3 and b0 == 0 and b1 == 0:
        cmd = "END"
    elif b2 & 0xC0 == 0xC0:
        cmd = "COLOR_CHANGE"
    elif b2 & 0x80:
        cmd = "JUMP"
    else:
        cmd = "STITCH"

    return dx, dy, cmd


def parse_stitches(data):
    """Parse stitch data from bytes 512 onwards. Returns list of (x, y, cmd)."""
    stitches = []
    x, y = 0.0, 0.0
    offset = 512
    color_index = 0
    normal_count = 0
    jump_count = 0

    while offset + 2 < len(data):
        b0, b1, b2 = data[offset], data[offset + 1], data[offset + 2]
        offset += 3

        dx, dy, cmd = decode_stitch(b0, b1, b2)

        if cmd == "END":
            break

        x += dx * 0.1  # convert to mm
        y += dy * 0.1

        if cmd == "COLOR_CHANGE":
            color_index += 1
        elif cmd == "STITCH":
            normal_count += 1
        elif cmd == "JUMP":
            jump_count += 1

        stitches.append((x, y, cmd, color_index))

    return stitches, normal_count, jump_count, color_index + 1


def render_preview(stitches, width_mm, height_mm, filename):
    """Render stitch preview as PNG image."""
    # Default colors for DST (no color info in file)
    colors = [
        (0, 0, 0), (255, 0, 0), (0, 128, 0), (0, 0, 255),
        (255, 165, 0), (128, 0, 128), (0, 128, 128), (128, 128, 0),
        (255, 0, 255), (0, 255, 255), (128, 64, 0), (64, 128, 0),
    ]

    if not stitches:
        return

    # Calculate bounds
    xs = [s[0] for s in stitches if s[2] == "STITCH"]
    ys = [s[1] for s in stitches if s[2] == "STITCH"]
    if not xs:
        return

    min_x, max_x = min(xs), max(xs)
    min_y, max_y = min(ys), max(ys)
    design_w = max_x - min_x
    design_h = max_y - min_y

    if design_w == 0 or design_h == 0:
        return

    # Scale to fit in 800x800 with margin
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
            color = colors[ci % len(colors)]
            draw.line([(prev_x, prev_y), (sx, sy)], fill=color, width=1)

        if cmd == "STITCH" or cmd == "JUMP":
            prev_x, prev_y = sx, sy
        elif cmd == "COLOR_CHANGE":
            prev_x, prev_y = None, None

    img.save(filename)


def process_dst(filepath):
    """Process a single DST file and return metadata dict."""
    basename = os.path.splitext(os.path.basename(filepath))[0]
    print(f"  Processing: {os.path.basename(filepath)}")

    with open(filepath, "rb") as f:
        data = f.read()

    header = parse_header(data)
    stitches, normal_count, jump_count, num_colors = parse_stitches(data)

    width_mm = (header["plus_x"] + header["minus_x"]) * 0.1
    height_mm = (header["plus_y"] + header["minus_y"]) * 0.1

    # Render preview
    preview_name = f"{basename}_preview.png"
    preview_path = os.path.join(OUTPUT_DIR, preview_name)
    render_preview(stitches, width_mm, height_mm, preview_path)

    info = {
        "filename": os.path.basename(filepath),
        "format": "DST (Tajima)",
        "label": header["label"],
        "stitch_count": normal_count,
        "jump_count": jump_count,
        "color_changes": header["color_changes"],
        "num_colors": num_colors,
        "colors": "Not stored in DST format",
        "width_mm": round(width_mm, 1),
        "height_mm": round(height_mm, 1),
        "file_size_bytes": len(data),
        "preview_image": preview_name,
    }

    # Save individual JSON
    json_path = os.path.join(OUTPUT_DIR, f"{basename}_info.json")
    with open(json_path, "w") as f:
        json.dump(info, f, indent=2, ensure_ascii=False)

    return info


def main():
    print("=== DST File Extraction ===")
    dst_files = sorted([
        os.path.join(EXAMPLE_DIR, f) for f in os.listdir(EXAMPLE_DIR)
        if f.lower().endswith(".dst")
    ])

    if not dst_files:
        print("No DST files found.")
        return

    all_info = []
    for fp in dst_files:
        info = process_dst(fp)
        all_info.append(info)
        print(f"    Label: {info['label']}")
        print(f"    Stitches: {info['stitch_count']}, Jumps: {info['jump_count']}")
        print(f"    Colors: {info['num_colors']}")
        print(f"    Size: {info['width_mm']} x {info['height_mm']} mm")
        print()

    # Save combined summary
    summary_path = os.path.join(OUTPUT_DIR, "dst_summary.json")
    with open(summary_path, "w") as f:
        json.dump(all_info, f, indent=2, ensure_ascii=False)
    print(f"Summary saved to: {summary_path}")


if __name__ == "__main__":
    main()
