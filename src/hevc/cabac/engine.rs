//! The CABAC arithmetic decoding engine (clause 9.3.4.3).

use super::ctx::{INIT_VALUES, NUM_CTX};
use super::tables::{RANGE_TAB_LPS, TRANS_IDX_LPS, TRANS_IDX_MPS};
use crate::hevc::error::{Error, Result};

/// Number of zero bytes the engine may synthesise past the end of a substream.
///
/// The nine-bit prefetch of `initialization` plus renormalisation legitimately
/// reads a little past the last meaningful byte of a slice segment.
const OVERREAD_SLACK: usize = 16;

/// Arithmetic decoder plus the flat array of context variables.
///
/// Context variables are packed as `pStateIdx << 1 | valMps`.
#[derive(Clone)]
pub struct Cabac<'a> {
    data: &'a [u8],
    /// Next bit to read, counted from the first bit of `data`.
    pos: usize,
    range: u32,
    offset: u32,
    /// `pStateIdx << 1 | valMps` for every context variable.
    pub ctx: [u8; NUM_CTX],
}

/// Derives every context variable from `SliceQpY` (clause 9.3.2.2).
///
/// Each entry is packed as `pStateIdx << 1 | valMps`.
pub fn initial_contexts(qp: i32) -> [u8; NUM_CTX] {
    let q = qp.clamp(0, 51);
    let mut out = [0u8; NUM_CTX];
    for (o, &init) in out.iter_mut().zip(INIT_VALUES.iter()) {
        let slope = (init >> 4) as i32 * 5 - 45;
        let offset = ((init & 15) as i32) * 8 - 16;
        let pre = (((slope * q) >> 4) + offset).clamp(1, 126);
        let mps = u8::from(pre > 63);
        let state = if mps == 1 { pre - 64 } else { 63 - pre };
        *o = ((state as u8) << 1) | mps;
    }
    out
}

impl<'a> Cabac<'a> {
    /// Creates an engine over `data` starting at byte `byte_off`.
    ///
    /// `qp` is `SliceQpY`, used to initialise the context variables.
    pub fn new(data: &'a [u8], byte_off: usize, qp: i32) -> Result<Cabac<'a>> {
        let mut c = Cabac {
            data,
            pos: byte_off * 8,
            range: 510,
            offset: 0,
            ctx: [0; NUM_CTX],
        };
        c.init_contexts(qp);
        c.init_engine()?;
        Ok(c)
    }

    /// Resets the context variables from the initialisation table (9.3.2.2).
    pub fn init_contexts(&mut self, qp: i32) {
        self.ctx = initial_contexts(qp);
    }

    /// Restarts the arithmetic decoder at the current byte position (9.3.2.5).
    pub fn init_engine(&mut self) -> Result<()> {
        self.pos = (self.pos + 7) & !7;
        self.range = 510;
        self.offset = self.read_bits(9)?;
        if self.offset >= 510 {
            return Err(Error::InvalidData("CABAC ivlOffset is 510 or 511"));
        }
        Ok(())
    }

    /// Repositions the engine at byte `byte_off` and restarts it.
    pub fn restart_at(&mut self, byte_off: usize) -> Result<()> {
        self.pos = byte_off * 8;
        self.init_engine()
    }

    /// Next byte boundary at or after the current position.
    ///
    /// `DecodeTerminate` consumes no bit when it returns 1 and the last bit it
    /// shifted in is the stop bit, so this is where the next substream starts.
    pub fn aligned_pos(&self) -> usize {
        (self.pos + 7) >> 3
    }

    #[inline]
    fn read_bit(&mut self) -> Result<u32> {
        let byte_idx = self.pos >> 3;
        let b = match self.data.get(byte_idx) {
            Some(&b) => b,
            None => {
                if byte_idx >= self.data.len() + OVERREAD_SLACK {
                    return Err(Error::Truncated);
                }
                0
            }
        };
        let bit = (b >> (7 - (self.pos & 7))) & 1;
        self.pos += 1;
        Ok(bit as u32)
    }

    #[inline]
    fn read_bits(&mut self, n: u32) -> Result<u32> {
        let mut v = 0u32;
        for _ in 0..n {
            v = (v << 1) | self.read_bit()?;
        }
        Ok(v)
    }

    /// `DecodeDecision`: decodes one bin with context variable `ctx_idx`.
    #[inline]
    pub fn decision(&mut self, ctx_idx: usize) -> Result<u32> {
        let s = self.ctx[ctx_idx];
        let state = (s >> 1) as usize;
        let mps = (s & 1) as u32;
        let q = ((self.range >> 6) & 3) as usize;
        let lps = RANGE_TAB_LPS[state][q] as u32;
        self.range -= lps;
        let bin;
        if self.offset >= self.range {
            bin = 1 - mps;
            self.offset -= self.range;
            self.range = lps;
            let new_mps = if state == 0 { 1 - mps } else { mps };
            self.ctx[ctx_idx] = (TRANS_IDX_LPS[state] << 1) | new_mps as u8;
        } else {
            bin = mps;
            self.ctx[ctx_idx] = (TRANS_IDX_MPS[state] << 1) | mps as u8;
        }
        while self.range < 256 {
            self.range <<= 1;
            self.offset = (self.offset << 1) | self.read_bit()?;
        }
        Ok(bin)
    }

    /// `DecodeBypass`: decodes one bin with the equiprobable model.
    #[inline]
    pub fn bypass(&mut self) -> Result<u32> {
        self.offset = (self.offset << 1) | self.read_bit()?;
        if self.offset >= self.range {
            self.offset -= self.range;
            Ok(1)
        } else {
            Ok(0)
        }
    }

    /// Decodes `n` bypass bins as an unsigned integer, most significant first.
    #[inline]
    pub fn bypass_bits(&mut self, n: u32) -> Result<u32> {
        let mut v = 0u32;
        for _ in 0..n {
            v = (v << 1) | self.bypass()?;
        }
        Ok(v)
    }

    /// `DecodeTerminate`: decodes `end_of_slice_segment_flag` and `pcm_flag`.
    #[inline]
    pub fn terminate(&mut self) -> Result<u32> {
        self.range -= 2;
        if self.offset >= self.range {
            Ok(1)
        } else {
            while self.range < 256 {
                self.range <<= 1;
                self.offset = (self.offset << 1) | self.read_bit()?;
            }
            Ok(0)
        }
    }

    /// Aligns to the next byte boundary after `DecodeTerminate` returned 1.
    ///
    /// Used before reading PCM samples, which are not arithmetically coded.
    pub fn align_after_terminate(&mut self) {
        // `DecodeTerminate` consumes no bit when it returns 1, and the last bit
        // shifted into ivlOffset is the alignment or stop bit, so the next
        // unread bit is already the first bit after it.
        self.pos = (self.pos + 7) & !7;
    }

    /// Reads `n` raw bits directly from the substream (PCM sample data).
    pub fn raw_bits(&mut self, n: u32) -> Result<u32> {
        self.read_bits(n)
    }
}
