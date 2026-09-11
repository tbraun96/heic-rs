#!/usr/bin/env python3
"""Generate the source PNGs for the heic-rs test fixtures.

Pure stdlib: no PIL, no numpy. Every pixel here is synthesised by this file,
so the fixtures carry no third-party image data.

Usage: python3 gen-pngs.py <output-directory>

All output is 8-bit truecolour RGB (PNG colour type 2), non-interlaced.
"""

import math
import os
import struct
import sys
import zlib

BIT_DEPTH = 8
COLOUR_TYPE_RGB = 2


def _chunk(tag: bytes, payload: bytes) -> bytes:
    """One PNG chunk: length, tag, payload, CRC32 over tag+payload."""
    return (
        struct.pack(">I", len(payload))
        + tag
        + payload
        + struct.pack(">I", zlib.crc32(tag + payload) & 0xFFFFFFFF)
    )


def write_png(path: str, width: int, height: int, rows: list) -> None:
    """Write `rows` (one bytes/bytearray of 3*width per row) as an RGB PNG."""
    if len(rows) != height:
        raise ValueError(f"{path}: expected {height} rows, got {len(rows)}")
    ihdr = struct.pack(
        ">IIBBBBB", width, height, BIT_DEPTH, COLOUR_TYPE_RGB, 0, 0, 0
    )
    raw = bytearray()
    for row in rows:
        if len(row) != width * 3:
            raise ValueError(f"{path}: bad row length {len(row)}")
        raw.append(0)  # filter type 0 (None) for every scanline
        raw += row
    idat = zlib.compress(bytes(raw), 9)
    blob = (
        b"\x89PNG\r\n\x1a\n"
        + _chunk(b"IHDR", ihdr)
        + _chunk(b"IDAT", idat)
        + _chunk(b"IEND", b"")
    )
    with open(path, "wb") as handle:
        handle.write(blob)
    print(f"  wrote {os.path.basename(path)} ({width}x{height}, {len(blob)} bytes)")


def solid(width: int, height: int, rgb: tuple) -> list:
    row = bytes(rgb) * width
    return [row] * height


def gradient(width: int, height: int) -> list:
    """Red ramps left->right, green ramps top->bottom, blue is constant 128."""
    rows = []
    for y in range(height):
        green = (y * 255) // (height - 1)
        row = bytearray(width * 3)
        for x in range(width):
            row[x * 3 + 0] = (x * 255) // (width - 1)
            row[x * 3 + 1] = green
            row[x * 3 + 2] = 128
        rows.append(bytes(row))
    return rows


def checker(size: int, cell: int) -> list:
    """Black/white checkerboard; the top-left cell is white."""
    white = b"\xff\xff\xff" * size
    black = b"\x00\x00\x00" * size
    white_first = bytearray()
    black_first = bytearray()
    for x in range(size):
        if (x // cell) % 2 == 0:
            white_first += b"\xff\xff\xff"
            black_first += b"\x00\x00\x00"
        else:
            white_first += b"\x00\x00\x00"
            black_first += b"\xff\xff\xff"
    del white, black
    rows = []
    for y in range(size):
        rows.append(bytes(white_first if (y // cell) % 2 == 0 else black_first))
    return rows


def strips(width: int, height: int, strip_w: int) -> list:
    """Vertical strips of pure red, pure green, pure blue (in that order)."""
    colours = [(255, 0, 0), (0, 255, 0), (0, 0, 255)]
    row = bytearray()
    for x in range(width):
        row += bytes(colours[min(x // strip_w, len(colours) - 1)])
    return [bytes(row)] * height


def photo(width: int, height: int) -> list:
    """A smooth, fully synthetic multi-colour field.

    Superposed sinusoids give large low-frequency areas plus gentle detail, so
    a HEVC encoder produces a realistic multi-tile image rather than a trivial
    flat one. No external data is used.
    """
    # Precompute per-column terms; the inner loop is the hot path.
    cos_x = [math.cos(x / 61.0) for x in range(width)]
    sin_x = [math.sin(x / 137.0) for x in range(width)]
    rows = []
    for y in range(height):
        sin_y = math.sin(y / 89.0)
        cos_y = math.cos(y / 173.0)
        ry = 96.0 * sin_y
        gy = 64.0 * cos_y
        row = bytearray(width * 3)
        for x in range(width):
            r = 128.0 + ry + 48.0 * cos_x[x]
            g = 128.0 + gy + 72.0 * sin_x[x] * sin_y
            b = 128.0 + 80.0 * math.sin((x + y) / 211.0) + 24.0 * cos_x[x] * cos_y
            row[x * 3 + 0] = 0 if r < 0 else (255 if r > 255 else int(r))
            row[x * 3 + 1] = 0 if g < 0 else (255 if g > 255 else int(g))
            row[x * 3 + 2] = 0 if b < 0 else (255 if b > 255 else int(b))
        rows.append(bytes(row))
    return rows


def main(argv: list) -> int:
    if len(argv) != 2:
        sys.stderr.write("usage: gen-pngs.py <output-directory>\n")
        return 2
    out = argv[1]
    os.makedirs(out, exist_ok=True)
    join = lambda name: os.path.join(out, name)

    write_png(join("flat-64.png"), 64, 64, solid(64, 64, (200, 30, 60)))
    write_png(join("flat-white-16.png"), 16, 16, solid(16, 16, (255, 255, 255)))
    write_png(join("gradient-512.png"), 512, 512, gradient(512, 512))
    write_png(join("checker-64.png"), 64, 64, checker(64, 8))
    write_png(join("checker-1024.png"), 1024, 1024, checker(1024, 64))
    write_png(join("rgb-strips-96.png"), 96, 32, strips(96, 32, 32))
    write_png(join("photo-2048.png"), 2048, 1536, photo(2048, 1536))
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
