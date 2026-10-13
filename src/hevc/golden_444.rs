//! Golden vector: a real 4:4:4 8-bit intra picture, decoded sample-exact.
//!
//! `tests/fixtures/hevc/intra-444-8.hevc` is a 128x128 frame of FFmpeg's
//! `testsrc2` pattern, encoded by x265 as a single intra picture in the RExt
//! profile with 4:4:4 chroma (`scripts/make-444-vector.sh`). It goes down to
//! 4x4 transform blocks, where 4:4:4 still signals `cbf_cb` and `cbf_cr` while
//! 4:2:0 and 4:2:2 inherit them from the parent.
//!
//! The expected checksum is of the same stream decoded by FFmpeg's libavcodec,
//! an independent decoder; the script prints it.

use super::decode_annexb;

const VECTOR: &[u8] = include_bytes!("../../tests/fixtures/hevc/intra-444-8.hevc");

/// FNV-1a over every sample of the planes (Y, then Cb, then Cr), one byte per
/// sample: the layout of FFmpeg's `yuv444p`.
const EXPECTED: u64 = 0x85d1_8677_3eb9_b943;

fn fnv1a(h: &mut u64, sample: u16) {
    let b = u8::try_from(sample).expect("an 8-bit sample");
    *h ^= u64::from(b);
    *h = h.wrapping_mul(0x0100_0000_01b3);
}

#[test]
fn a_444_8bit_picture_decodes_sample_exact() {
    let f = decode_annexb(VECTOR).expect("decode");
    assert_eq!((f.width, f.height, f.bit_depth), (128, 128, 8));
    assert_eq!(f.chroma, crate::hevc::ChromaFormat::Yuv444);
    let (w, h) = (f.width as usize, f.height as usize);
    let mut sum = 0xcbf2_9ce4_8422_2325u64;
    for (plane, stride) in [(&f.y, f.y_stride), (&f.cb, f.c_stride), (&f.cr, f.c_stride)] {
        for y in 0..h {
            for x in 0..w {
                fnv1a(&mut sum, plane[y * stride as usize + x]);
            }
        }
    }
    assert_eq!(
        sum, EXPECTED,
        "4:4:4 picture differs from the reference decode"
    );
}
