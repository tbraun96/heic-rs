#!/usr/bin/env bash
# Grid vectors for two camera and phone layouts `sips` cannot write (tests/fixtures):
#
# - grid-422-10bit.heic: a 3 x 2 grid of 64 x 64 tiles, 4:2:2 10-bit, as Canon and Sony `.HIF`
#   files are. Before the Table 8-3 fix it decoded with wrong colour.
# - full-range-no-colr.heic: a 2 x 2 grid of 4:2:0 10-bit tiles whose only colour description is
#   the full-range VUI of their bitstream: no `colr` at all, as some phones write. Read as limited
#   range, its near-black and near-white quarters are crushed and clipped.
#
# Each `.ref.png` is Apple's decode (`sips`), the ground truth, as for the other fixtures.
#
# Needs macOS (`sips`), python3 and libheif's `heif-enc` with x265 (`brew install libheif`).
# Encoders differ between versions, so the committed `.heic` files are the reference; rerun only
# to regenerate them all.
set -euo pipefail
cd "$(dirname "$0")/.."
out=tests/fixtures
tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT

python3 scripts/gen-grid-pngs.py "$tmp"

heif-enc -b 10 -p chroma=422 --cut-tiles 64 -q 90 "$tmp/grid-422-10bit.png" -o "$out/grid-422-10bit.heic"

# Full range, written in the nclx `colr` and in the VUI; then the `colr` box is renamed `free`
# (same size, nothing else moves), which leaves the VUI as the only description.
heif-enc -b 10 -p chroma=420 --cut-tiles 64 -q 90 --full_range_flag 1 --matrix_coefficients 1 \
  --colour_primaries 1 --transfer_characteristic 13 "$tmp/full-range-no-colr.png" -o "$tmp/full-range-nclx.heic"
python3 - "$tmp/full-range-nclx.heic" "$out/full-range-no-colr.heic" <<'EOF'
import sys
b = bytearray(open(sys.argv[1], "rb").read())
i = b.find(b"colrnclx")
assert i > 0 and b.find(b"colrnclx", i + 8) < 0, "expected exactly one nclx colr box"
b[i:i + 4] = b"free"
open(sys.argv[2], "wb").write(b)
EOF

for name in grid-422-10bit full-range-no-colr; do
  sips -s format png "$out/$name.heic" --out "$out/$name.ref.png" >/dev/null
done
echo "wrote $out/grid-422-10bit.{heic,ref.png} and $out/full-range-no-colr.{heic,ref.png}"
