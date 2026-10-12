//! Tests for the one thing the VUI is read for: `video_signal_type`.

use super::parse;
use crate::hevc::bits::{BitReader, BitWriter};
use crate::hevc::frame::VideoSignal;
use crate::hevc::synth;

/// Writes a VUI that has nothing in it but, when asked, `video_signal_type`.
fn vui(signal: Option<(bool, Option<[u8; 3]>)>) -> alloc::vec::Vec<u8> {
    let mut w = BitWriter::default();
    w.put(0, 1); // aspect_ratio_info_present_flag
    w.put(0, 1); // overscan_info_present_flag
    w.put(u32::from(signal.is_some()), 1); // video_signal_type_present_flag
    if let Some((full_range, description)) = signal {
        w.put(5, 3); // video_format: unspecified
        w.put(u32::from(full_range), 1); // video_full_range_flag
        w.put(u32::from(description.is_some()), 1); // colour_description_present_flag
        for code in description.into_iter().flatten() {
            w.put(u32::from(code), 8);
        }
    }
    w.put(0, 1); // chroma_loc_info_present_flag
    w.put(0, 3); // neutral_chroma_indication, field_seq, frame_field_info
    w.put(0, 1); // default_display_window_flag
    w.put(0, 1); // vui_timing_info_present_flag
    w.put(0, 1); // bitstream_restriction_flag
    w.put(0b101, 3); // what follows the VUI in these tests
    w.rbsp_trailing();
    w.bytes
}

/// Parses a VUI and checks that the reader stops exactly behind it.
fn parsed(bytes: &[u8]) -> Option<VideoSignal> {
    let mut r = BitReader::new(bytes);
    let signal = parse(&mut r, 0).expect("the VUI parses");
    assert_eq!(r.u(3).expect("bits behind the VUI"), 0b101);
    signal
}

#[test]
fn a_vui_without_a_signal_type_says_nothing_about_colour() {
    assert_eq!(parsed(&vui(None)), None);
}

#[test]
fn the_range_is_read_even_without_a_colour_description() {
    let full = VideoSignal {
        full_range: true,
        description: None,
    };
    assert_eq!(parsed(&vui(Some((true, None)))), Some(full));
    let limited = VideoSignal {
        full_range: false,
        description: None,
    };
    assert_eq!(parsed(&vui(Some((false, None)))), Some(limited));
}

#[test]
fn the_colour_description_is_primaries_transfer_matrix() {
    // BT.601 primaries, BT.601 transfer and matrix, full range: a phone photo.
    let signal = VideoSignal {
        full_range: true,
        description: Some([5, 6, 6]),
    };
    assert_eq!(parsed(&vui(Some((true, Some([5, 6, 6]))))), Some(signal));
}

#[test]
fn a_stream_without_a_vui_has_no_signal() {
    let (sets, _slice) = synth::picture(64, 64);
    let sets: alloc::vec::Vec<&[u8]> = sets.iter().map(|s| s.as_slice()).collect();
    assert_eq!(crate::hevc::video_signal(&sets).expect("parses"), None);
}
