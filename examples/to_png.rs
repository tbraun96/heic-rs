//! Decode a HEIF file and write it out as a PNG.
//!
//! ```text
//! cargo run --example to_png -- in.heic out.png
//! ```
//!
//! The PNG writer here is deliberately tiny and dependency-free: it emits
//! stored (uncompressed) deflate blocks, so the output is valid but large.
//! It exists to make the decoder's output viewable, not to be a good encoder.

use heic_rs::{DecodeOptions, PixelLayout};

fn main() {
    let mut args = std::env::args().skip(1);
    let (Some(input), Some(output)) = (args.next(), args.next()) else {
        eprintln!("usage: to_png <in.heic> <out.png>");
        std::process::exit(2);
    };
    let options = DecodeOptions::default().with_layout(PixelLayout::Rgb8);
    let image = match heic_rs::io::decode_file(&input, &options) {
        Ok(i) => i,
        Err(e) => {
            eprintln!("{input}: {e}");
            std::process::exit(1);
        }
    };
    let png = encode_png(&image.data, image.width, image.height);
    if let Err(e) = std::fs::write(&output, png) {
        eprintln!("{output}: {e}");
        std::process::exit(1);
    }
    println!("wrote {output} ({}x{})", image.width, image.height);
}

fn encode_png(rgb: &[u8], width: u32, height: u32) -> Vec<u8> {
    let mut raw = Vec::with_capacity((width as usize * 3 + 1) * height as usize);
    for y in 0..height as usize {
        raw.push(0);
        let start = y * width as usize * 3;
        raw.extend_from_slice(&rgb[start..start + width as usize * 3]);
    }
    let mut out = Vec::new();
    out.extend_from_slice(&[0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a]);
    let mut ihdr = Vec::new();
    ihdr.extend_from_slice(&width.to_be_bytes());
    ihdr.extend_from_slice(&height.to_be_bytes());
    ihdr.extend_from_slice(&[8, 2, 0, 0, 0]);
    chunk(&mut out, b"IHDR", &ihdr);
    chunk(&mut out, b"IDAT", &zlib_stored(&raw));
    chunk(&mut out, b"IEND", &[]);
    out
}

fn chunk(out: &mut Vec<u8>, kind: &[u8; 4], body: &[u8]) {
    out.extend_from_slice(&(body.len() as u32).to_be_bytes());
    out.extend_from_slice(kind);
    out.extend_from_slice(body);
    let mut crc_input = Vec::with_capacity(4 + body.len());
    crc_input.extend_from_slice(kind);
    crc_input.extend_from_slice(body);
    out.extend_from_slice(&crc32(&crc_input).to_be_bytes());
}

fn zlib_stored(data: &[u8]) -> Vec<u8> {
    let mut out = vec![0x78, 0x01];
    for (i, block) in data.chunks(65_535).enumerate() {
        let last = (i + 1) * 65_535 >= data.len();
        out.push(u8::from(last));
        out.extend_from_slice(&(block.len() as u16).to_le_bytes());
        out.extend_from_slice(&(!(block.len() as u16)).to_le_bytes());
        out.extend_from_slice(block);
    }
    out.extend_from_slice(&adler32(data).to_be_bytes());
    out
}

fn adler32(data: &[u8]) -> u32 {
    let (mut a, mut b) = (1u32, 0u32);
    for &byte in data {
        a = (a + u32::from(byte)) % 65_521;
        b = (b + a) % 65_521;
    }
    (b << 16) | a
}

fn crc32(data: &[u8]) -> u32 {
    let mut crc = 0xffff_ffffu32;
    for &byte in data {
        crc ^= u32::from(byte);
        for _ in 0..8 {
            let mask = 0u32.wrapping_sub(crc & 1);
            crc = (crc >> 1) ^ (0xedb8_8320 & mask);
        }
    }
    !crc
}
