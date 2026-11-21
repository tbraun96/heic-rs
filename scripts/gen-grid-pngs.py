#!/usr/bin/env python3
"""Source PNGs for the grid vectors made by `make-grid-vectors.sh`.

Pure standard library (`zlib`, `struct`, `math`, `sys`, `os`): every pixel is written here,
16 bits per sample, so the encoder gets a 10-bit-clean input. The content is smooth on purpose:
these vectors check the 4:2:2 grid path and the bitstream's sample range, not edge handling.

    python3 scripts/gen-grid-pngs.py <out-dir>
"""
import math
import os
import struct
import sys
import zlib


def png16(path, w, h, pixel):
    """Writes a 16-bit RGB PNG whose pixel (x, y) is `pixel(x, y)`, three floats in 0..1."""
    rows = bytearray()
    for y in range(h):
        rows.append(0)  # filter: none
        for x in range(w):
            for v in pixel(x, y):
                rows += struct.pack(">H", max(0, min(65535, round(v * 65535))))

    def chunk(kind, data):
        return struct.pack(">I", len(data)) + kind + data + struct.pack(">I", zlib.crc32(kind + data) & 0xFFFFFFFF)

    ihdr = struct.pack(">IIBBBBB", w, h, 16, 2, 0, 0, 0)
    with open(path, "wb") as f:
        f.write(b"\x89PNG\r\n\x1a\n" + chunk(b"IHDR", ihdr) + chunk(b"IDAT", zlib.compress(bytes(rows), 9)) + chunk(b"IEND", b""))


def grid_422(x, y):
    """192 x 128: slow colour waves, 3 x 2 tiles of 64."""
    return (
        0.5 + 0.35 * math.sin(x / 31.0 + 0.3),
        0.5 + 0.35 * math.sin(y / 23.0 + 1.1),
        0.5 + 0.35 * math.cos((x + y) / 41.0),
    )


def full_range(x, y):
    """128 x 128: near black (8/255) and near white (248/255) on top, a grey ramp and a colour
    ramp below: values a limited-range reading crushes or clips."""
    if y < 64:
        v = 8 / 255 if x < 64 else 248 / 255
        return (v, v, v)
    if x < 64:
        v = (16 + 224 * x / 63) / 255
        return (v, v, v)
    t = (x - 64) / 63
    return (0.15 + 0.7 * t, 0.6 - 0.3 * t, 0.85 - 0.6 * t)


if __name__ == "__main__":
    out = sys.argv[1]
    os.makedirs(out, exist_ok=True)
    png16(os.path.join(out, "grid-422-10bit.png"), 192, 128, grid_422)
    png16(os.path.join(out, "full-range-no-colr.png"), 128, 128, full_range)
