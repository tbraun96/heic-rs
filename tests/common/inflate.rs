//! A textbook canonical-Huffman deflate decoder, for the test PNG reader.
//!
//! Stored, fixed and dynamic blocks; no dependency, no `unsafe`, and every
//! failure is a `String` rather than a panic so that a corrupt fixture is
//! reported rather than crashing the test binary.

struct Bits<'a> {
    data: &'a [u8],
    pos: usize,
    bit: u32,
    acc: u32,
}

impl<'a> Bits<'a> {
    fn take(&mut self, n: u32) -> Result<u32, String> {
        while self.bit < n {
            let byte = *self
                .data
                .get(self.pos)
                .ok_or_else(|| "deflate underrun".to_string())?;
            self.pos += 1;
            self.acc |= u32::from(byte) << self.bit;
            self.bit += 8;
        }
        let v = self.acc & ((1u32 << n) - 1);
        self.acc >>= n;
        self.bit -= n;
        Ok(v)
    }
}

struct Huff {
    counts: [u16; 16],
    symbols: Vec<u16>,
}

fn build(lengths: &[u8]) -> Huff {
    let mut counts = [0u16; 16];
    for &l in lengths {
        counts[l as usize] += 1;
    }
    counts[0] = 0;
    let mut offs = [0u16; 16];
    for i in 1..16 {
        offs[i] = offs[i - 1] + counts[i - 1];
    }
    let mut symbols = vec![0u16; lengths.len()];
    for (sym, &l) in lengths.iter().enumerate() {
        if l != 0 {
            symbols[offs[l as usize] as usize] = sym as u16;
            offs[l as usize] += 1;
        }
    }
    Huff { counts, symbols }
}

fn decode_sym(b: &mut Bits<'_>, h: &Huff) -> Result<u16, String> {
    let (mut code, mut first, mut index) = (0i32, 0i32, 0i32);
    for len in 1..16 {
        code |= b.take(1)? as i32;
        let count = i32::from(h.counts[len]);
        if code - count < first {
            return h
                .symbols
                .get((index + (code - first)) as usize)
                .copied()
                .ok_or_else(|| "bad huffman symbol".to_string());
        }
        index += count;
        first = (first + count) << 1;
        code <<= 1;
    }
    Err("oversized huffman code".into())
}

const LEN_BASE: [u16; 29] = [
    3, 4, 5, 6, 7, 8, 9, 10, 11, 13, 15, 17, 19, 23, 27, 31, 35, 43, 51, 59, 67, 83, 99, 115, 131,
    163, 195, 227, 258,
];
const LEN_EXTRA: [u32; 29] = [
    0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 1, 1, 2, 2, 2, 2, 3, 3, 3, 3, 4, 4, 4, 4, 5, 5, 5, 5, 0,
];
const DIST_BASE: [u16; 30] = [
    1, 2, 3, 4, 5, 7, 9, 13, 17, 25, 33, 49, 65, 97, 129, 193, 257, 385, 513, 769, 1025, 1537,
    2049, 3073, 4097, 6145, 8193, 12289, 16385, 24577,
];
const DIST_EXTRA: [u32; 30] = [
    0, 0, 0, 0, 1, 1, 2, 2, 3, 3, 4, 4, 5, 5, 6, 6, 7, 7, 8, 8, 9, 9, 10, 10, 11, 11, 12, 12, 13,
    13,
];

pub fn inflate(data: &[u8]) -> Result<Vec<u8>, String> {
    let mut b = Bits {
        data,
        pos: 0,
        bit: 0,
        acc: 0,
    };
    let mut out = Vec::new();
    loop {
        let last = b.take(1)?;
        match b.take(2)? {
            0 => {
                b.acc = 0;
                b.bit = 0;
                let len = u16::from_le_bytes([data[b.pos], data[b.pos + 1]]) as usize;
                b.pos += 4;
                out.extend_from_slice(
                    data.get(b.pos..b.pos + len)
                        .ok_or_else(|| "short block".to_string())?,
                );
                b.pos += len;
            }
            1 => {
                let mut lengths = vec![8u8; 288];
                lengths[144..256].fill(9);
                lengths[256..280].fill(7);
                block(&mut b, &build(&lengths), &build(&[5u8; 30]), &mut out)?;
            }
            2 => {
                let hlit = b.take(5)? as usize + 257;
                let hdist = b.take(5)? as usize + 1;
                let hclen = b.take(4)? as usize + 4;
                const ORDER: [usize; 19] = [
                    16, 17, 18, 0, 8, 7, 9, 6, 10, 5, 11, 4, 12, 3, 13, 2, 14, 1, 15,
                ];
                let mut cl = [0u8; 19];
                for &o in ORDER.iter().take(hclen) {
                    cl[o] = b.take(3)? as u8;
                }
                let clh = build(&cl);
                let mut lengths = Vec::with_capacity(hlit + hdist);
                while lengths.len() < hlit + hdist {
                    let sym = decode_sym(&mut b, &clh)?;
                    match sym {
                        0..=15 => lengths.push(sym as u8),
                        16 => {
                            let prev = *lengths.last().ok_or_else(|| "no previous".to_string())?;
                            let n = 3 + b.take(2)? as usize;
                            lengths.resize(lengths.len() + n, prev);
                        }
                        17 => {
                            let n = 3 + b.take(3)? as usize;
                            lengths.resize(lengths.len() + n, 0);
                        }
                        _ => {
                            let n = 11 + b.take(7)? as usize;
                            lengths.resize(lengths.len() + n, 0);
                        }
                    }
                }
                let lit = build(&lengths[..hlit]);
                let dist = build(&lengths[hlit..hlit + hdist]);
                block(&mut b, &lit, &dist, &mut out)?;
            }
            _ => return Err("reserved deflate block type".into()),
        }
        if last == 1 {
            return Ok(out);
        }
    }
}

fn block(b: &mut Bits<'_>, lit: &Huff, dist: &Huff, out: &mut Vec<u8>) -> Result<(), String> {
    loop {
        let sym = decode_sym(b, lit)?;
        match sym {
            0..=255 => out.push(sym as u8),
            256 => return Ok(()),
            _ => {
                let i = sym as usize - 257;
                if i >= 29 {
                    return Err("bad length symbol".into());
                }
                let len = LEN_BASE[i] as usize + b.take(LEN_EXTRA[i])? as usize;
                let d = decode_sym(b, dist)? as usize;
                if d >= 30 {
                    return Err("bad distance symbol".into());
                }
                let back = DIST_BASE[d] as usize + b.take(DIST_EXTRA[d])? as usize;
                if back > out.len() {
                    return Err("distance before the start of the stream".into());
                }
                let start = out.len() - back;
                for k in 0..len {
                    out.push(out[start + k]);
                }
            }
        }
    }
}
