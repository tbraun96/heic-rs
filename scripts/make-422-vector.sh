#!/usr/bin/env bash
# Regenerate tests/fixtures/hevc/intra-422-10.hevc and print its reference checksum.
#
#   bash scripts/make-422-vector.sh
#
# Apple's `sips` (scripts/make-fixtures.sh) cannot write 4:2:2 HEVC, so this one
# vector is made with FFmpeg: its synthetic `testsrc2` pattern (no third-party
# image data), encoded by x265 as one intra picture in 4:2:2 10-bit, and decoded
# back by FFmpeg's own HEVC decoder for the ground truth. Paste the printed
# checksum into src/hevc/golden_422.rs. Needs ffmpeg built with libx265, and python3.
set -euo pipefail

root_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
out="${root_dir}/tests/fixtures/hevc/intra-422-10.hevc"
tmp_dir="$(mktemp -d "${TMPDIR:-/tmp}/heic-rs-422.XXXXXX")"
trap 'rm -rf "${tmp_dir}"' EXIT
mkdir -p "$(dirname -- "${out}")"

ffmpeg -v error -y -f lavfi -i "testsrc2=size=128x128:rate=1,format=yuv422p10le" -frames:v 1 \
  -c:v libx265 -pix_fmt yuv422p10le -crf 8 \
  -x265-params "keyint=1:log-level=error:sao=1:aq-mode=0:rd=6:rdoq-level=2:tu-intra-depth=3" \
  -f hevc "${out}"
ffmpeg -v error -y -f hevc -i "${out}" -f rawvideo -pix_fmt yuv422p10le "${tmp_dir}/ref.yuv"

python3 - "${tmp_dir}/ref.yuv" <<'PY'
import sys
h = 0xCBF29CE484222325
for b in open(sys.argv[1], "rb").read():
    h = ((h ^ b) * 0x100000001B3) & 0xFFFFFFFFFFFFFFFF
print(f"{sys.argv[1]}: FNV-1a 0x{h:016x}")
PY
