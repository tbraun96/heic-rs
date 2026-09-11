# HEIC fixture ground truth

Machine-extracted from the bytes of every `tests/fixtures/*.heic`, produced by
`scripts/make-fixtures.sh` on macOS 27.0 (build 26A5425a) with Apple `sips`.
Regenerate the fixtures with `bash scripts/make-fixtures.sh`; if the host macOS
changes its HEVC encoder these numbers can move, so re-derive this file too.

Every value below is a literal read out of the file and is safe to turn into an
exact assertion.

## Invariants shared by all 9 fixtures

| Field | Value |
|---|---|
| `ftyp` major_brand | `heic` |
| `ftyp` minor_version | `0` |
| `ftyp` compatible_brands | `mif1`, `MiPr`, `miaf`, `MiHB`, `heic` (exactly 5, in this order) |
| `ftyp` box size / offset | 36 bytes at offset 0 |
| top-level boxes | `ftyp`, `meta`, `mdat` (in that order, nothing else) |
| `meta` | FullBox version 0, flags 0 |
| `hdlr` handler_type | `pict` (pre_defined 0, reserved 0, name empty) |
| `dinf`/`dref` | 1 entry, `url ` with flags 1 (self-contained) |
| `pitm` version | 0 (16-bit item_ID) |
| `iinf` version | 0 (16-bit entry_count) |
| `infe` version | 2 (16-bit item_ID), item_name always the empty string |
| `ipma` version / flags | 0 / 0 -> 8-bit property indices, essential = high bit `0x80` |
| `colr` | `nclx`, colour_primaries=**2**, transfer_characteristics=**2**, matrix_coefficients=**6**, full_range_flag=**1** (byte `0x80`) |
| `clli` | present, max_content_light_level=**203**, max_pic_average_light_level=**64** |
| `pixi` | num_channels=**3**, bits_per_channel = **8, 8, 8** |
| `irot` | **present in every fixture, angle == 0** (0 degrees) |
| `imir` | **absent everywhere** |
| `auxC` / alpha auxiliary item | **absent everywhere** (no fixture has alpha) |
| `pasp`, `clap`, `rICC`, `prof` | **absent everywhere** |
| `hvcC` | configurationVersion=1, general_profile_space=0, general_tier_flag=0, general_profile_idc=**3** (Main Still Picture), chroma_format_idc=**1** (4:2:0), bit_depth_luma/chroma=**8**/**8**, numOfArrays=**3** (VPS+SPS+PPS), lengthSizeMinusOne=**3** (4-byte NAL length prefix) |
| all `.ref.png` | 8-bit, PNG colour type 2 (truecolour RGB), non-interlaced |

Two gotchas worth encoding as tests:

* `irot` is **always present with angle 0**. A parser that treats "irot present"
  as "image is rotated" will be wrong on every one of these files.
* `sips -r 90` **re-encodes the pixels** rather than setting `irot`. `rotated-90.heic`
  therefore has a transposed grid (4 rows x 3 columns, 1536x2048 output) and an
  `irot` angle of 0 -- it is a *grid-transposition* fixture, not an `irot` fixture.
  None of these fixtures exercises a non-zero `irot` or any `imir`.

## Per-fixture summary

| Fixture | bytes | primary id | primary type | grid | ispe (primary) | tile ispe | items | `iloc` ver | ctor methods | Exif item | XMP `mime` item |
|---|---|---|---|---|---|---|---|---|---|---|---|
| `flat-white-16.heic` | 485 | 1 | `hvc1` | no | 16x16 | n/a | 1 | 0 | 0 | none | none |
| `flat-64.heic` | 488 | 1 | `hvc1` | no | 64x64 | n/a | 1 | 0 | 0 | none | none |
| `checker-64.heic` | 679 | 1 | `hvc1` | no | 64x64 | n/a | 1 | 0 | 0 | none | none |
| `rgb-strips-96.heic` | 527 | 1 | `hvc1` | no | 96x32 | n/a | 1 | 0 | 0 | none | none |
| `gradient-512.heic` | 4378 | 1 | `hvc1` | no | 512x512 | n/a | 1 | 0 | 0 | none | none |
| `checker-1024.heic` | 2566 | 5 | `grid` | 1024x1024 (2 rows x 2 cols) | 1024x1024 | 512x512 | 5 | 1 | 0+1 | none | none |
| `photo-2048.heic` | 55438 | 13 | `grid` | 2048x1536 (3 rows x 4 cols) | 2048x1536 | 512x512 | 13 | 1 | 0+1 | none | none |
| `rotated-90.heic` | 63758 | 13 | `grid` | 1536x2048 (4 rows x 3 cols) | 1536x2048 | 512x512 | 14 | 1 | 0+1 | id 14 | none |
| `with-exif.heic` | 60616 | 13 | `grid` | 2048x1536 (3 rows x 4 cols) | 2048x1536 | 512x512 | 15 | 1 | 0+1 | id 14 | id 15 |

---

## `flat-white-16.heic`

16x16 solid white (255,255,255). 485 bytes.

Top-level boxes (type@offset+size): `ftyp@0+36`, `meta@36+390`, `mdat@426+59`

### Items (`iinf`, entry_count = 1)

| item_ID | item_type | infe flags | hidden | notes |
|---|---|---|---|---|
| 1 | `hvc1` | 0 | no | **primary item** (`pitm` = 1) |

`pitm` primary item ID = **1** (type `hvc1`).

### `iref`

No `iref` box at all.

### Grid

The primary item is **not** a `grid` -- it is a single `hvc1` coded image. No `dimg` reference exists.

### `ipco` property array (1-based indices as used by `ipma`)

| index | box | values |
|---|---|---|
| 1 | `colr` | `nclx`, primaries=2, transfer=2, matrix=6, full_range=1 |
| 2 | `clli` | max_content_light_level=203, max_pic_average_light_level=64 |
| 3 | `ispe` | width=16, height=16 |
| 4 | `irot` | angle=0 (0 deg CCW) |
| 5 | `pixi` | num_channels=3, bits=8/8/8 |
| 6 | `hvcC` | profile_idc=3, tier=0, level_idc=30, chroma=1, bitdepth=8/8, numOfArrays=3, lengthSizeMinusOne=3 (payload 105 bytes) |

### `ipma` associations (version 0, flags 0)

| item_ID | property indices (essential marked `*`) | resolved |
|---|---|---|
| 1 | 1* 2 3 5 6* 4* | `colr` `clli` `ispe` `pixi` `hvcC` `irot` |

### `iloc` (version 0, offset_size=4, length_size=4, base_offset_size=0, index_size=0)

Version 0 has **no** `construction_method` field; every extent is implicitly
construction_method 0 (file offset).

| item_ID | construction_method | data_reference_index | base_offset | extent offset | extent length |
|---|---|---|---|---|---|
| 1 | 0 (file offset) | 0 | 0 | 442 | 43 |

`mdat` payload occupies file bytes [442, 485).

### Exif item

**No `Exif` item** in this file.

**Alpha auxiliary item:** none (`auxC` does not appear in `ipco`).

---

## `flat-64.heic`

64x64 solid RGB (200,30,60). 488 bytes.

Top-level boxes (type@offset+size): `ftyp@0+36`, `meta@36+389`, `mdat@425+63`

### Items (`iinf`, entry_count = 1)

| item_ID | item_type | infe flags | hidden | notes |
|---|---|---|---|---|
| 1 | `hvc1` | 0 | no | **primary item** (`pitm` = 1) |

`pitm` primary item ID = **1** (type `hvc1`).

### `iref`

No `iref` box at all.

### Grid

The primary item is **not** a `grid` -- it is a single `hvc1` coded image. No `dimg` reference exists.

### `ipco` property array (1-based indices as used by `ipma`)

| index | box | values |
|---|---|---|
| 1 | `colr` | `nclx`, primaries=2, transfer=2, matrix=6, full_range=1 |
| 2 | `clli` | max_content_light_level=203, max_pic_average_light_level=64 |
| 3 | `ispe` | width=64, height=64 |
| 4 | `irot` | angle=0 (0 deg CCW) |
| 5 | `pixi` | num_channels=3, bits=8/8/8 |
| 6 | `hvcC` | profile_idc=3, tier=0, level_idc=30, chroma=1, bitdepth=8/8, numOfArrays=3, lengthSizeMinusOne=3 (payload 104 bytes) |

### `ipma` associations (version 0, flags 0)

| item_ID | property indices (essential marked `*`) | resolved |
|---|---|---|
| 1 | 1* 2 3 5 6* 4* | `colr` `clli` `ispe` `pixi` `hvcC` `irot` |

### `iloc` (version 0, offset_size=4, length_size=4, base_offset_size=0, index_size=0)

Version 0 has **no** `construction_method` field; every extent is implicitly
construction_method 0 (file offset).

| item_ID | construction_method | data_reference_index | base_offset | extent offset | extent length |
|---|---|---|---|---|---|
| 1 | 0 (file offset) | 0 | 0 | 441 | 47 |

`mdat` payload occupies file bytes [441, 488).

### Exif item

**No `Exif` item** in this file.

**Alpha auxiliary item:** none (`auxC` does not appear in `ipco`).

---

## `checker-64.heic`

64x64 8px black/white checkerboard, top-left cell white. 679 bytes.

Top-level boxes (type@offset+size): `ftyp@0+36`, `meta@36+389`, `mdat@425+254`

### Items (`iinf`, entry_count = 1)

| item_ID | item_type | infe flags | hidden | notes |
|---|---|---|---|---|
| 1 | `hvc1` | 0 | no | **primary item** (`pitm` = 1) |

`pitm` primary item ID = **1** (type `hvc1`).

### `iref`

No `iref` box at all.

### Grid

The primary item is **not** a `grid` -- it is a single `hvc1` coded image. No `dimg` reference exists.

### `ipco` property array (1-based indices as used by `ipma`)

| index | box | values |
|---|---|---|
| 1 | `colr` | `nclx`, primaries=2, transfer=2, matrix=6, full_range=1 |
| 2 | `clli` | max_content_light_level=203, max_pic_average_light_level=64 |
| 3 | `ispe` | width=64, height=64 |
| 4 | `irot` | angle=0 (0 deg CCW) |
| 5 | `pixi` | num_channels=3, bits=8/8/8 |
| 6 | `hvcC` | profile_idc=3, tier=0, level_idc=30, chroma=1, bitdepth=8/8, numOfArrays=3, lengthSizeMinusOne=3 (payload 104 bytes) |

### `ipma` associations (version 0, flags 0)

| item_ID | property indices (essential marked `*`) | resolved |
|---|---|---|
| 1 | 1* 2 3 5 6* 4* | `colr` `clli` `ispe` `pixi` `hvcC` `irot` |

### `iloc` (version 0, offset_size=4, length_size=4, base_offset_size=0, index_size=0)

Version 0 has **no** `construction_method` field; every extent is implicitly
construction_method 0 (file offset).

| item_ID | construction_method | data_reference_index | base_offset | extent offset | extent length |
|---|---|---|---|---|---|
| 1 | 0 (file offset) | 0 | 0 | 441 | 238 |

`mdat` payload occupies file bytes [441, 679).

### Exif item

**No `Exif` item** in this file.

**Alpha auxiliary item:** none (`auxC` does not appear in `ipco`).

---

## `rgb-strips-96.heic`

96x32, three 32px vertical strips: red, green, blue. 527 bytes.

Top-level boxes (type@offset+size): `ftyp@0+36`, `meta@36+390`, `mdat@426+101`

### Items (`iinf`, entry_count = 1)

| item_ID | item_type | infe flags | hidden | notes |
|---|---|---|---|---|
| 1 | `hvc1` | 0 | no | **primary item** (`pitm` = 1) |

`pitm` primary item ID = **1** (type `hvc1`).

### `iref`

No `iref` box at all.

### Grid

The primary item is **not** a `grid` -- it is a single `hvc1` coded image. No `dimg` reference exists.

### `ipco` property array (1-based indices as used by `ipma`)

| index | box | values |
|---|---|---|
| 1 | `colr` | `nclx`, primaries=2, transfer=2, matrix=6, full_range=1 |
| 2 | `clli` | max_content_light_level=203, max_pic_average_light_level=64 |
| 3 | `ispe` | width=96, height=32 |
| 4 | `irot` | angle=0 (0 deg CCW) |
| 5 | `pixi` | num_channels=3, bits=8/8/8 |
| 6 | `hvcC` | profile_idc=3, tier=0, level_idc=30, chroma=1, bitdepth=8/8, numOfArrays=3, lengthSizeMinusOne=3 (payload 105 bytes) |

### `ipma` associations (version 0, flags 0)

| item_ID | property indices (essential marked `*`) | resolved |
|---|---|---|
| 1 | 1* 2 3 5 6* 4* | `colr` `clli` `ispe` `pixi` `hvcC` `irot` |

### `iloc` (version 0, offset_size=4, length_size=4, base_offset_size=0, index_size=0)

Version 0 has **no** `construction_method` field; every extent is implicitly
construction_method 0 (file offset).

| item_ID | construction_method | data_reference_index | base_offset | extent offset | extent length |
|---|---|---|---|---|---|
| 1 | 0 (file offset) | 0 | 0 | 442 | 85 |

`mdat` payload occupies file bytes [442, 527).

### Exif item

**No `Exif` item** in this file.

**Alpha auxiliary item:** none (`auxC` does not appear in `ipco`).

---

## `gradient-512.heic`

512x512 red ramp left->right, green ramp top->bottom, blue=128. 4378 bytes.

Top-level boxes (type@offset+size): `ftyp@0+36`, `meta@36+389`, `mdat@425+3953`

### Items (`iinf`, entry_count = 1)

| item_ID | item_type | infe flags | hidden | notes |
|---|---|---|---|---|
| 1 | `hvc1` | 0 | no | **primary item** (`pitm` = 1) |

`pitm` primary item ID = **1** (type `hvc1`).

### `iref`

No `iref` box at all.

### Grid

The primary item is **not** a `grid` -- it is a single `hvc1` coded image. No `dimg` reference exists.

### `ipco` property array (1-based indices as used by `ipma`)

| index | box | values |
|---|---|---|
| 1 | `colr` | `nclx`, primaries=2, transfer=2, matrix=6, full_range=1 |
| 2 | `clli` | max_content_light_level=203, max_pic_average_light_level=64 |
| 3 | `ispe` | width=512, height=512 |
| 4 | `irot` | angle=0 (0 deg CCW) |
| 5 | `pixi` | num_channels=3, bits=8/8/8 |
| 6 | `hvcC` | profile_idc=3, tier=0, level_idc=90, chroma=1, bitdepth=8/8, numOfArrays=3, lengthSizeMinusOne=3 (payload 104 bytes) |

### `ipma` associations (version 0, flags 0)

| item_ID | property indices (essential marked `*`) | resolved |
|---|---|---|
| 1 | 1* 2 3 5 6* 4* | `colr` `clli` `ispe` `pixi` `hvcC` `irot` |

### `iloc` (version 0, offset_size=4, length_size=4, base_offset_size=0, index_size=0)

Version 0 has **no** `construction_method` field; every extent is implicitly
construction_method 0 (file offset).

| item_ID | construction_method | data_reference_index | base_offset | extent offset | extent length |
|---|---|---|---|---|---|
| 1 | 0 (file offset) | 0 | 0 | 441 | 3937 |

`mdat` payload occupies file bytes [441, 4378).

### Exif item

**No `Exif` item** in this file.

**Alpha auxiliary item:** none (`auxC` does not appear in `ipco`).

---

## `checker-1024.heic`

1024x1024 64px black/white checkerboard, top-left cell white. 2566 bytes.

Top-level boxes (type@offset+size): `ftyp@0+36`, `meta@36+634`, `mdat@670+1896`

### Items (`iinf`, entry_count = 5)

| item_ID | item_type | infe flags | hidden | notes |
|---|---|---|---|---|
| 1 | `hvc1` | 1 | yes | grid tile |
| 2 | `hvc1` | 1 | yes | grid tile |
| 3 | `hvc1` | 1 | yes | grid tile |
| 4 | `hvc1` | 1 | yes | grid tile |
| 5 | `grid` | 0 | no | **primary item** (`pitm` = 5) |

`pitm` primary item ID = **5** (type `grid`).

### `iref` (version 0, 16-bit IDs)

| ref type | from_item | to_item_IDs (ordered) |
|---|---|---|
| `dimg` | 5 | 1, 2, 3, 4 |

### Grid (`grid` item payload, 8 bytes, read via `iloc` construction_method 1 from `idat`)

| Field | Value |
|---|---|
| raw payload | `0000010104000400` |
| version | 0 |
| flags | 0 (0 -> 16-bit output dimensions) |
| rows_minus_one | 1 -> **2 rows** |
| columns_minus_one | 1 -> **2 columns** |
| output_width | **1024** |
| output_height | **1024** |
| ordered tile item IDs (`dimg`) | 1, 2, 3, 4 |
| tile count | 4 (= 2 x 2) |

Tiles are laid out row-major. Tile `ispe` is **512x512** for every tile, so the
tiled canvas is 1024x1024 and is cropped to the grid output size 1024x1024.

### `ipco` property array (1-based indices as used by `ipma`)

| index | box | values |
|---|---|---|
| 1 | `colr` | `nclx`, primaries=2, transfer=2, matrix=6, full_range=1 |
| 2 | `clli` | max_content_light_level=203, max_pic_average_light_level=64 |
| 3 | `ispe` | width=512, height=512 |
| 4 | `ispe` | width=1024, height=1024 |
| 5 | `irot` | angle=0 (0 deg CCW) |
| 6 | `pixi` | num_channels=3, bits=8/8/8 |
| 7 | `hvcC` | profile_idc=3, tier=0, level_idc=90, chroma=1, bitdepth=8/8, numOfArrays=3, lengthSizeMinusOne=3 (payload 104 bytes) |

### `ipma` associations (version 0, flags 0)

| item_ID | property indices (essential marked `*`) | resolved |
|---|---|---|
| 1-4 (identical) | 3* 1* 2 7* | `ispe` `colr` `clli` `hvcC` |
| 5 | 1* 2 4 6 5* | `colr` `clli` `ispe` `pixi` `irot` |

### `iloc` (version 1, offset_size=4, length_size=4, base_offset_size=0, index_size=0)

Version 1 carries an explicit 4-bit `construction_method`.

| item_ID | construction_method | data_reference_index | base_offset | extent offset | extent length |
|---|---|---|---|---|---|
| 1 | 0 (file offset) | 0 | 0 | 686 | 470 |
| 2 | 0 (file offset) | 0 | 0 | 1156 | 470 |
| 3 | 0 (file offset) | 0 | 0 | 1626 | 470 |
| 4 | 0 (file offset) | 0 | 0 | 2096 | 470 |
| 5 | 1 (`idat`) | 0 | 0 | 0 | 8 |

`idat` box payload occupies file bytes [566, 574) (8 bytes) and holds only the grid descriptor.

`mdat` payload occupies file bytes [686, 2566).

### Exif item

**No `Exif` item** in this file.

**Alpha auxiliary item:** none (`auxC` does not appear in `ipco`).

---

## `photo-2048.heic`

2048x1536 smooth synthetic sinusoid field. 55438 bytes.

Top-level boxes (type@offset+size): `ftyp@0+36`, `meta@36+1002`, `mdat@1038+54400`

### Items (`iinf`, entry_count = 13)

| item_ID | item_type | infe flags | hidden | notes |
|---|---|---|---|---|
| 1 | `hvc1` | 1 | yes | grid tile |
| 2 | `hvc1` | 1 | yes | grid tile |
| 3 | `hvc1` | 1 | yes | grid tile |
| 4 | `hvc1` | 1 | yes | grid tile |
| 5 | `hvc1` | 1 | yes | grid tile |
| 6 | `hvc1` | 1 | yes | grid tile |
| 7 | `hvc1` | 1 | yes | grid tile |
| 8 | `hvc1` | 1 | yes | grid tile |
| 9 | `hvc1` | 1 | yes | grid tile |
| 10 | `hvc1` | 1 | yes | grid tile |
| 11 | `hvc1` | 1 | yes | grid tile |
| 12 | `hvc1` | 1 | yes | grid tile |
| 13 | `grid` | 0 | no | **primary item** (`pitm` = 13) |

`pitm` primary item ID = **13** (type `grid`).

### `iref` (version 0, 16-bit IDs)

| ref type | from_item | to_item_IDs (ordered) |
|---|---|---|
| `dimg` | 13 | 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12 |

### Grid (`grid` item payload, 8 bytes, read via `iloc` construction_method 1 from `idat`)

| Field | Value |
|---|---|
| raw payload | `0000020308000600` |
| version | 0 |
| flags | 0 (0 -> 16-bit output dimensions) |
| rows_minus_one | 2 -> **3 rows** |
| columns_minus_one | 3 -> **4 columns** |
| output_width | **2048** |
| output_height | **1536** |
| ordered tile item IDs (`dimg`) | 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12 |
| tile count | 12 (= 3 x 4) |

Tiles are laid out row-major. Tile `ispe` is **512x512** for every tile, so the
tiled canvas is 2048x1536 and is cropped to the grid output size 2048x1536.

### `ipco` property array (1-based indices as used by `ipma`)

| index | box | values |
|---|---|---|
| 1 | `colr` | `nclx`, primaries=2, transfer=2, matrix=6, full_range=1 |
| 2 | `clli` | max_content_light_level=203, max_pic_average_light_level=64 |
| 3 | `ispe` | width=512, height=512 |
| 4 | `ispe` | width=2048, height=1536 |
| 5 | `irot` | angle=0 (0 deg CCW) |
| 6 | `pixi` | num_channels=3, bits=8/8/8 |
| 7 | `hvcC` | profile_idc=3, tier=0, level_idc=90, chroma=1, bitdepth=8/8, numOfArrays=3, lengthSizeMinusOne=3 (payload 104 bytes) |

### `ipma` associations (version 0, flags 0)

| item_ID | property indices (essential marked `*`) | resolved |
|---|---|---|
| 1-12 (identical) | 3* 1* 2 7* | `ispe` `colr` `clli` `hvcC` |
| 13 | 1* 2 4 6 5* | `colr` `clli` `ispe` `pixi` `irot` |

### `iloc` (version 1, offset_size=4, length_size=4, base_offset_size=0, index_size=0)

Version 1 carries an explicit 4-bit `construction_method`.

| item_ID | construction_method | data_reference_index | base_offset | extent offset | extent length |
|---|---|---|---|---|---|
| 1 | 0 (file offset) | 0 | 0 | 1054 | 3896 |
| 2 | 0 (file offset) | 0 | 0 | 4950 | 5146 |
| 3 | 0 (file offset) | 0 | 0 | 10096 | 4681 |
| 4 | 0 (file offset) | 0 | 0 | 14777 | 4102 |
| 5 | 0 (file offset) | 0 | 0 | 18879 | 4357 |
| 6 | 0 (file offset) | 0 | 0 | 23236 | 4886 |
| 7 | 0 (file offset) | 0 | 0 | 28122 | 5187 |
| 8 | 0 (file offset) | 0 | 0 | 33309 | 4805 |
| 9 | 0 (file offset) | 0 | 0 | 38114 | 3678 |
| 10 | 0 (file offset) | 0 | 0 | 41792 | 4980 |
| 11 | 0 (file offset) | 0 | 0 | 46772 | 4657 |
| 12 | 0 (file offset) | 0 | 0 | 51429 | 4009 |
| 13 | 1 (`idat`) | 0 | 0 | 0 | 8 |

`idat` box payload occupies file bytes [806, 814) (8 bytes) and holds only the grid descriptor.

`mdat` payload occupies file bytes [1054, 55438).

### Exif item

**No `Exif` item** in this file.

**Alpha auxiliary item:** none (`auxC` does not appear in `ipco`).

---

## `rotated-90.heic`

photo-2048.heic put through `sips -r 90`. 63758 bytes.

Top-level boxes (type@offset+size): `ftyp@0+36`, `meta@36+1053`, `mdat@1089+62669`

### Items (`iinf`, entry_count = 14)

| item_ID | item_type | infe flags | hidden | notes |
|---|---|---|---|---|
| 1 | `hvc1` | 1 | yes | grid tile |
| 2 | `hvc1` | 1 | yes | grid tile |
| 3 | `hvc1` | 1 | yes | grid tile |
| 4 | `hvc1` | 1 | yes | grid tile |
| 5 | `hvc1` | 1 | yes | grid tile |
| 6 | `hvc1` | 1 | yes | grid tile |
| 7 | `hvc1` | 1 | yes | grid tile |
| 8 | `hvc1` | 1 | yes | grid tile |
| 9 | `hvc1` | 1 | yes | grid tile |
| 10 | `hvc1` | 1 | yes | grid tile |
| 11 | `hvc1` | 1 | yes | grid tile |
| 12 | `hvc1` | 1 | yes | grid tile |
| 13 | `grid` | 0 | no | **primary item** (`pitm` = 13) |
| 14 | `Exif` | 1 | yes | EXIF metadata item |

`pitm` primary item ID = **13** (type `grid`).

### `iref` (version 0, 16-bit IDs)

| ref type | from_item | to_item_IDs (ordered) |
|---|---|---|
| `dimg` | 13 | 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12 |
| `cdsc` | 14 | 13 |

### Grid (`grid` item payload, 8 bytes, read via `iloc` construction_method 1 from `idat`)

| Field | Value |
|---|---|
| raw payload | `0000030206000800` |
| version | 0 |
| flags | 0 (0 -> 16-bit output dimensions) |
| rows_minus_one | 3 -> **4 rows** |
| columns_minus_one | 2 -> **3 columns** |
| output_width | **1536** |
| output_height | **2048** |
| ordered tile item IDs (`dimg`) | 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12 |
| tile count | 12 (= 4 x 3) |

Tiles are laid out row-major. Tile `ispe` is **512x512** for every tile, so the
tiled canvas is 1536x2048 and is cropped to the grid output size 1536x2048.

### `ipco` property array (1-based indices as used by `ipma`)

| index | box | values |
|---|---|---|
| 1 | `colr` | `nclx`, primaries=2, transfer=2, matrix=6, full_range=1 |
| 2 | `clli` | max_content_light_level=203, max_pic_average_light_level=64 |
| 3 | `ispe` | width=512, height=512 |
| 4 | `ispe` | width=1536, height=2048 |
| 5 | `irot` | angle=0 (0 deg CCW) |
| 6 | `pixi` | num_channels=3, bits=8/8/8 |
| 7 | `hvcC` | profile_idc=3, tier=0, level_idc=90, chroma=1, bitdepth=8/8, numOfArrays=3, lengthSizeMinusOne=3 (payload 104 bytes) |

### `ipma` associations (version 0, flags 0)

| item_ID | property indices (essential marked `*`) | resolved |
|---|---|---|
| 1-12 (identical) | 3* 1* 2 7* | `ispe` `colr` `clli` `hvcC` |
| 13 | 1* 2 4 6 5* | `colr` `clli` `ispe` `pixi` `irot` |

### `iloc` (version 1, offset_size=4, length_size=4, base_offset_size=0, index_size=0)

Version 1 carries an explicit 4-bit `construction_method`.

| item_ID | construction_method | data_reference_index | base_offset | extent offset | extent length |
|---|---|---|---|---|---|
| 1 | 0 (file offset) | 0 | 0 | 1165 | 4136 |
| 2 | 0 (file offset) | 0 | 0 | 5301 | 5050 |
| 3 | 0 (file offset) | 0 | 0 | 10351 | 4500 |
| 4 | 0 (file offset) | 0 | 0 | 14851 | 5778 |
| 5 | 0 (file offset) | 0 | 0 | 20629 | 5601 |
| 6 | 0 (file offset) | 0 | 0 | 26230 | 6070 |
| 7 | 0 (file offset) | 0 | 0 | 32300 | 5216 |
| 8 | 0 (file offset) | 0 | 0 | 37516 | 5839 |
| 9 | 0 (file offset) | 0 | 0 | 43355 | 5462 |
| 10 | 0 (file offset) | 0 | 0 | 48817 | 4653 |
| 11 | 0 (file offset) | 0 | 0 | 53470 | 5556 |
| 12 | 0 (file offset) | 0 | 0 | 59026 | 4732 |
| 13 | 1 (`idat`) | 0 | 0 | 0 | 8 |
| 14 | 0 (file offset) | 0 | 0 | 1105 | 60 |

`idat` box payload occupies file bytes [841, 849) (8 bytes) and holds only the grid descriptor.

`mdat` payload occupies file bytes [1105, 63758).

### Exif item 14

| Field | Value |
|---|---|
| iloc extent | offset 1105, length 60 (construction_method 0) |
| `iref` link | `cdsc` from 14 to 13 |
| first 4 bytes | `00000006` = exif_tiff_header_offset **6** |
| bytes 4..10 | `457869660000` (= `Exif\0\0`) |
| TIFF header at byte 10 | `4d4d002a00000008` = big-endian (`MM`), magic 42, IFD0 offset 8 |
| IFD0 entry count | 3 |
| leading 48 bytes | `000000064578696600004d4d002a00000008000301120003000000010001000001420004000000010000020001430004` |

**Alpha auxiliary item:** none (`auxC` does not appear in `ipco`).

---

## `with-exif.heic`

photo-2048.heic with description/copyright/artist set via `sips -s`. 60616 bytes.

Top-level boxes (type@offset+size): `ftyp@0+36`, `meta@36+1124`, `mdat@1160+59456`

### Items (`iinf`, entry_count = 15)

| item_ID | item_type | infe flags | hidden | notes |
|---|---|---|---|---|
| 1 | `hvc1` | 1 | yes | grid tile |
| 2 | `hvc1` | 1 | yes | grid tile |
| 3 | `hvc1` | 1 | yes | grid tile |
| 4 | `hvc1` | 1 | yes | grid tile |
| 5 | `hvc1` | 1 | yes | grid tile |
| 6 | `hvc1` | 1 | yes | grid tile |
| 7 | `hvc1` | 1 | yes | grid tile |
| 8 | `hvc1` | 1 | yes | grid tile |
| 9 | `hvc1` | 1 | yes | grid tile |
| 10 | `hvc1` | 1 | yes | grid tile |
| 11 | `hvc1` | 1 | yes | grid tile |
| 12 | `hvc1` | 1 | yes | grid tile |
| 13 | `grid` | 0 | no | **primary item** (`pitm` = 13) |
| 14 | `Exif` | 1 | yes | EXIF metadata item |
| 15 | `mime` | 1 | yes | content_type = `application/rdf+xml` (XMP packet) |

`pitm` primary item ID = **13** (type `grid`).

### `iref` (version 0, 16-bit IDs)

| ref type | from_item | to_item_IDs (ordered) |
|---|---|---|
| `dimg` | 13 | 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12 |
| `cdsc` | 14 | 13 |
| `cdsc` | 15 | 13 |

### Grid (`grid` item payload, 8 bytes, read via `iloc` construction_method 1 from `idat`)

| Field | Value |
|---|---|
| raw payload | `0000020308000600` |
| version | 0 |
| flags | 0 (0 -> 16-bit output dimensions) |
| rows_minus_one | 2 -> **3 rows** |
| columns_minus_one | 3 -> **4 columns** |
| output_width | **2048** |
| output_height | **1536** |
| ordered tile item IDs (`dimg`) | 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12 |
| tile count | 12 (= 3 x 4) |

Tiles are laid out row-major. Tile `ispe` is **512x512** for every tile, so the
tiled canvas is 2048x1536 and is cropped to the grid output size 2048x1536.

### `ipco` property array (1-based indices as used by `ipma`)

| index | box | values |
|---|---|---|
| 1 | `colr` | `nclx`, primaries=2, transfer=2, matrix=6, full_range=1 |
| 2 | `clli` | max_content_light_level=203, max_pic_average_light_level=64 |
| 3 | `ispe` | width=512, height=512 |
| 4 | `ispe` | width=2048, height=1536 |
| 5 | `irot` | angle=0 (0 deg CCW) |
| 6 | `pixi` | num_channels=3, bits=8/8/8 |
| 7 | `hvcC` | profile_idc=3, tier=0, level_idc=90, chroma=1, bitdepth=8/8, numOfArrays=3, lengthSizeMinusOne=3 (payload 104 bytes) |

### `ipma` associations (version 0, flags 0)

| item_ID | property indices (essential marked `*`) | resolved |
|---|---|---|
| 1-12 (identical) | 3* 1* 2 7* | `ispe` `colr` `clli` `hvcC` |
| 13 | 1* 2 4 6 5* | `colr` `clli` `ispe` `pixi` `irot` |

### `iloc` (version 1, offset_size=4, length_size=4, base_offset_size=0, index_size=0)

Version 1 carries an explicit 4-bit `construction_method`.

| item_ID | construction_method | data_reference_index | base_offset | extent offset | extent length |
|---|---|---|---|---|---|
| 1 | 0 (file offset) | 0 | 0 | 2097 | 4169 |
| 2 | 0 (file offset) | 0 | 0 | 6266 | 5347 |
| 3 | 0 (file offset) | 0 | 0 | 11613 | 4928 |
| 4 | 0 (file offset) | 0 | 0 | 16541 | 4498 |
| 5 | 0 (file offset) | 0 | 0 | 21039 | 4827 |
| 6 | 0 (file offset) | 0 | 0 | 25866 | 5271 |
| 7 | 0 (file offset) | 0 | 0 | 31137 | 5710 |
| 8 | 0 (file offset) | 0 | 0 | 36847 | 5244 |
| 9 | 0 (file offset) | 0 | 0 | 42091 | 3990 |
| 10 | 0 (file offset) | 0 | 0 | 46081 | 5316 |
| 11 | 0 (file offset) | 0 | 0 | 51397 | 5008 |
| 12 | 0 (file offset) | 0 | 0 | 56405 | 4211 |
| 13 | 1 (`idat`) | 0 | 0 | 0 | 8 |
| 14 | 0 (file offset) | 0 | 0 | 1176 | 164 |
| 15 | 0 (file offset) | 0 | 0 | 1340 | 757 |

`idat` box payload occupies file bytes [896, 904) (8 bytes) and holds only the grid descriptor.

`mdat` payload occupies file bytes [1176, 60616).

### Exif item 14

| Field | Value |
|---|---|
| iloc extent | offset 1176, length 164 (construction_method 0) |
| `iref` link | `cdsc` from 14 to 13 |
| first 4 bytes | `00000006` = exif_tiff_header_offset **6** |
| bytes 4..10 | `457869660000` (= `Exif\0\0`) |
| TIFF header at byte 10 | `4d4d002a00000008` = big-endian (`MM`), magic 42, IFD0 offset 8 |
| IFD0 entry count | 6 |
| leading 48 bytes | `000000064578696600004d4d002a000000080006010e00020000001000000056011200030000000100010000013b0002` |

**Alpha auxiliary item:** none (`auxC` does not appear in `ipco`).

---

## Reference decode spot checks

Sampled straight out of the `.ref.png` files (Apple's decode of our `.heic`), so
these are the RGB values a correct decoder should land on. HEVC here is lossy, so
compare with a small tolerance rather than for equality -- except the two files
that survive the round trip exactly.

| Fixture | pixel (x, y) | reference RGB | source PNG RGB |
|---|---|---|---|
| `flat-white-16.ref.png` | (0,0), (8,8), (15,15) | 255,255,255 (all three) | 255,255,255 |
| `flat-64.ref.png` | (0,0) | 198, 31, 59 | 200, 30, 60 |
| `flat-64.ref.png` | (32,32) | 198, 31, 59 | 200, 30, 60 |
| `flat-64.ref.png` | (63,63) | 197, 31, 61 | 200, 30, 60 |
| `checker-64.ref.png` | (4,4) / (12,12) | 255,255,255 | 255,255,255 |
| `checker-64.ref.png` | (12,4) / (4,12) | 0,0,0 | 0,0,0 |
| `checker-1024.ref.png` | (32,32), (992,992) | 255,255,255 | 255,255,255 |
| `checker-1024.ref.png` | (96,32) | 0,0,0 | 0,0,0 |
| `rgb-strips-96.ref.png` | (16,16) | 253, 1, 0 | 255, 0, 0 |
| `rgb-strips-96.ref.png` | (48,16) | 2, 254, 2 | 0, 255, 0 |
| `rgb-strips-96.ref.png` | (80,16) | 0, 1, 253 | 0, 0, 255 |
| `gradient-512.ref.png` | (0,0) | 1, 1, 126 | 0, 0, 128 |
| `gradient-512.ref.png` | (511,0) | 252, 0, 127 | 255, 0, 128 |
| `gradient-512.ref.png` | (0,511) | 0, 254, 130 | 0, 255, 128 |
| `gradient-512.ref.png` | (511,511) | 254, 254, 130 | 255, 255, 128 |
| `gradient-512.ref.png` | (256,256) | 127, 127, 125 | 128, 128, 128 |

`flat-white-16`, `checker-64` and `checker-1024` come back bit-exact at the sampled
points; the rest drift by up to 3 codes per channel from 4:2:0 subsampling and
quantisation. A sensible tolerance for the smooth fixtures is +/- 4 per channel.
