#!/usr/bin/env bash
# Regenerate every file in tests/fixtures/ from PNGs we synthesise ourselves.
#
#   bash scripts/make-fixtures.sh
#
# Source PNGs come from scripts/gen-pngs.py (pure stdlib Python, no third-party
# image data). Each PNG is encoded to HEIC with Apple's `sips`, and each HEIC is
# decoded back to PNG with `sips` as well -- that decode is the ground truth our
# decoder is compared against. Safe to re-run: every output is rewritten.
set -euo pipefail

script_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
root_dir="$(cd -- "${script_dir}/.." && pwd)"
fixtures_dir="${root_dir}/tests/fixtures"
gen_pngs="${script_dir}/gen-pngs.py"

command -v sips >/dev/null 2>&1 || { echo "error: sips not found (macOS only)" >&2; exit 1; }
command -v python3 >/dev/null 2>&1 || { echo "error: python3 not found" >&2; exit 1; }
[ -f "${gen_pngs}" ] || { echo "error: missing ${gen_pngs}" >&2; exit 1; }

tmp_dir="$(mktemp -d "${TMPDIR:-/tmp}/heic-rs-fixtures.XXXXXX")"
cleanup() { rm -rf "${tmp_dir}"; }
trap cleanup EXIT

mkdir -p "${fixtures_dir}"

echo "==> synthesising source PNGs in ${tmp_dir}"
python3 "${gen_pngs}" "${tmp_dir}"

# quiet wrapper: sips is chatty on success and we only care about failures
run_sips() {
  if ! out="$(sips "$@" 2>&1)"; then
    echo "error: sips $* failed:" >&2
    echo "${out}" >&2
    return 1
  fi
}

encode_pair() {
  # $1 = source png, $2 = fixture basename (no extension)
  local src="$1" name="$2"
  local heic="${fixtures_dir}/${name}.heic"
  local ref="${fixtures_dir}/${name}.ref.png"
  rm -f "${heic}" "${ref}"
  run_sips -s format heic "${src}" --out "${heic}"
  run_sips -s format png "${heic}" --out "${ref}"
  echo "  ${name}.heic + ${name}.ref.png"
}

echo "==> encoding HEIC fixtures and Apple-decoded references"
for png in "${tmp_dir}"/*.png; do
  encode_pair "${png}" "$(basename "${png}" .png)"
done

base_heic="${fixtures_dir}/photo-2048.heic"
[ -f "${base_heic}" ] || { echo "error: ${base_heic} was not produced" >&2; exit 1; }

echo "==> rotation variant"
rot="${fixtures_dir}/rotated-90.heic"
rm -f "${rot}" "${fixtures_dir}/rotated-90.ref.png"
cp "${base_heic}" "${rot}"
run_sips -r 90 "${rot}"
run_sips -s format png "${rot}" --out "${fixtures_dir}/rotated-90.ref.png"
echo "  rotated-90.heic + rotated-90.ref.png"

echo "==> EXIF variant"
exif="${fixtures_dir}/with-exif.heic"
rm -f "${exif}" "${fixtures_dir}/with-exif.ref.png"
cp "${base_heic}" "${exif}"
exif_ok=0
if sips -s description "heic-rs fixture" \
        -s copyright "MIT OR Apache-2.0" \
        -s artist "heic-rs scripts/make-fixtures.sh" \
        "${exif}" >/dev/null 2>&1; then
  # Only keep it if sips really wrote an Exif item into the meta box.
  if python3 - "${exif}" <<'PY'
import sys
blob = open(sys.argv[1], "rb").read(1 << 16)
sys.exit(0 if b"Exif" in blob else 1)
PY
  then
    exif_ok=1
  fi
fi
if [ "${exif_ok}" -eq 1 ]; then
  run_sips -s format png "${exif}" --out "${fixtures_dir}/with-exif.ref.png"
  echo "  with-exif.heic + with-exif.ref.png"
else
  rm -f "${exif}"
  echo "  NOTE: sips on this system cannot inject an Exif item into HEIC;"
  echo "        with-exif.heic was SKIPPED (no fixture was faked)."
fi

echo
echo "==> fixtures in ${fixtures_dir}"
printf '%-28s %6s %7s %12s\n' FILE WIDTH HEIGHT BYTES
printf '%-28s %6s %7s %12s\n' ---------------------------- ------ ------- ------------
for f in "${fixtures_dir}"/*.heic "${fixtures_dir}"/*.ref.png; do
  [ -e "${f}" ] || continue
  w="$(sips -g pixelWidth "${f}" 2>/dev/null | awk '/pixelWidth/{print $2}')"
  h="$(sips -g pixelHeight "${f}" 2>/dev/null | awk '/pixelHeight/{print $2}')"
  sz="$(wc -c <"${f}" | tr -d ' ')"
  printf '%-28s %6s %7s %12s\n' "$(basename "${f}")" "${w:-?}" "${h:-?}" "${sz}"
done
echo
echo "done."
