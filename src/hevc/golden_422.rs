//! Golden vector: a real 4:2:2 10-bit intra picture, decoded sample-exact.
//!
//! `tests/fixtures/hevc/intra-422-10.hevc` is a 128x128 frame of FFmpeg's
//! `testsrc2` pattern, encoded by x265 as a single intra picture in the RExt
//! profile with 4:2:2 chroma (`scripts/make-422-vector.sh`). It exercises the
//! 4:2:2 chroma path end to end: the Table 8-3 mode mapping, the two stacked
//! chroma transform blocks per transform unit, SAO and deblocking.
//!
//! The expected checksum is of the same stream decoded by FFmpeg's libavcodec,
//! an independent decoder; the script prints it.

use super::decode_annexb;

const VECTOR: &[u8] = include_bytes!("../../tests/fixtures/hevc/intra-422-10.hevc");

/// FNV-1a over every sample of the planes (Y, then Cb, then Cr), each sample as
/// two little-endian bytes: the layout of FFmpeg's `yuv422p10le`.
const EXPECTED: u64 = 0xc932_4cae_793f_082f;

fn fnv1a(h: &mut u64, sample: u16) {
    for b in sample.to_le_bytes() {
        *h ^= u64::from(b);
        *h = h.wrapping_mul(0x0100_0000_01b3);
    }
}

#[test]
fn a_422_10bit_picture_decodes_sample_exact() {
    let f = decode_annexb(VECTOR).expect("decode");
    assert_eq!((f.width, f.height, f.bit_depth), (128, 128, 10));
    assert_eq!(f.chroma, crate::hevc::ChromaFormat::Yuv422);
    let (w, h) = (f.width as usize, f.height as usize);
    let mut sum = 0xcbf2_9ce4_8422_2325u64;
    for y in 0..h {
        for x in 0..w {
            fnv1a(&mut sum, f.y[y * f.y_stride as usize + x]);
        }
    }
    for plane in [&f.cb, &f.cr] {
        for y in 0..h {
            for x in 0..w / 2 {
                fnv1a(&mut sum, plane[y * f.c_stride as usize + x]);
            }
        }
    }
    assert_eq!(
        sum, EXPECTED,
        "4:2:2 picture differs from the reference decode"
    );
}
