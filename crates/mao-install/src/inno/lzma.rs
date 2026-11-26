//! A streaming LZMA1 decoder for the streams inside an Inno Setup package: five bytes of properties (the `lc/lp/pb`
//! byte and the dictionary size), then the raw range-coded stream with no size field. The stream may end with the
//! end marker (Inno Setup's compressor writes one) or simply stop being read once the caller has what it wanted.
//!
//! Memory use is the dictionary (8 MB in Funcom's package) plus a 64 KiB input buffer, whatever the stream's length.
//! The algorithm is Igor Pavlov's LZMA as described in the LZMA SDK's specification (public domain).

use std::io::{self, Read};

const STATES: usize = 12;
const POS_STATES_MAX: usize = 16;
const LEN_LOW: usize = 8;
const LEN_HIGH: usize = 256;
const MATCH_MIN_LEN: usize = 2;
const MATCH_MAX_LEN: usize = 273;
const END_POS_MODEL_INDEX: u32 = 14;
const FULL_DISTANCES: usize = 128;
const PROB_INIT: u16 = 1024;
const TOP: u32 = 1 << 24;

// Where each group of probabilities starts in the one table.
const IS_MATCH: usize = 0;
const IS_REP: usize = IS_MATCH + STATES * POS_STATES_MAX;
const IS_REP_G0: usize = IS_REP + STATES;
const IS_REP_G1: usize = IS_REP_G0 + STATES;
const IS_REP_G2: usize = IS_REP_G1 + STATES;
const IS_REP0_LONG: usize = IS_REP_G2 + STATES;
const POS_SLOT: usize = IS_REP0_LONG + STATES * POS_STATES_MAX;
const POS_SPECIAL: usize = POS_SLOT + 4 * 64;
const ALIGN: usize = POS_SPECIAL + 1 + FULL_DISTANCES - END_POS_MODEL_INDEX as usize;
const LEN_CODER: usize = ALIGN + 16;
const LEN_CODER_SIZE: usize = 2 + 2 * POS_STATES_MAX * LEN_LOW + LEN_HIGH;
const REP_LEN_CODER: usize = LEN_CODER + LEN_CODER_SIZE;
const LITERAL: usize = REP_LEN_CODER + LEN_CODER_SIZE;

const INPUT_BUFFER: usize = 1 << 16;
/// How much is decoded before it is handed out (a match may run 273 bytes past it).
const BATCH: usize = 1 << 16;

/// Reads the unpacked bytes of one LZMA1 stream from `inner`. Reading stops (returns 0) at the stream's end marker;
/// a stream cut short gives `UnexpectedEof`.
pub struct Lzma1Reader<R: Read> {
    inner: R,
    input: Box<[u8]>,
    input_pos: usize,
    input_len: usize,
    /// The input ran out while the range decoder wanted a byte (it was given a zero).
    input_ended: bool,
    input_error: Option<io::Error>,

    range: u32,
    code: u32,
    probs: Vec<u16>,
    lc: u32,
    lp_mask: u64,
    pb_mask: u64,
    dict_size: u64,

    /// The dictionary, used as a ring: `total % dict.len()` is where the next byte goes.
    dict: Vec<u8>,
    dict_pos: usize,
    /// Bytes decoded so far, and how many of them the caller has been given.
    total: u64,
    given: u64,
    state: usize,
    reps: [u32; 4],
    finished: bool,
}

impl<R: Read> Lzma1Reader<R> {
    /// Reads the five property bytes and the range decoder's first five bytes from `inner`.
    pub fn new(mut inner: R) -> io::Result<Self> {
        let mut props = [0u8; 5];
        inner.read_exact(&mut props)?;
        let d = props[0] as u32;
        if d >= 9 * 5 * 5 {
            return Err(bad("LZMA properties byte out of range"));
        }
        let (lc, lp, pb) = (d % 9, (d / 9) % 5, d / 45);
        let dict_size = u32::from_le_bytes([props[1], props[2], props[3], props[4]]);
        // (the reference decoder also refuses to allocate more than the stream asks for; a ring of at least
        // 2 * BATCH keeps undelivered bytes from being overwritten)
        if dict_size > (1 << 30) {
            return Err(bad("LZMA dictionary larger than 1 GiB"));
        }
        let ring = (dict_size as usize).max(2 * BATCH + MATCH_MAX_LEN);
        let mut reader = Lzma1Reader {
            inner,
            input: vec![0u8; INPUT_BUFFER].into_boxed_slice(),
            input_pos: 0,
            input_len: 0,
            input_ended: false,
            input_error: None,
            range: 0xffff_ffff,
            code: 0,
            probs: vec![PROB_INIT; LITERAL + (0x300usize << (lc + lp))],
            lc,
            lp_mask: (1u64 << lp) - 1,
            pb_mask: (1u64 << pb) - 1,
            dict_size: dict_size.max(1) as u64,
            dict: vec![0u8; ring],
            dict_pos: 0,
            total: 0,
            given: 0,
            state: 0,
            reps: [0; 4],
            finished: false,
        };
        // The first byte of a range-coded stream is always zero.
        if reader.next_byte() != 0 {
            return Err(bad("LZMA stream does not start with a zero byte"));
        }
        for _ in 0..4 {
            reader.code = (reader.code << 8) | reader.next_byte() as u32;
        }
        reader.check_input()?;
        Ok(reader)
    }

    /// Whether the end marker has been read.
    #[cfg(test)]
    pub fn finished(&self) -> bool {
        self.finished
    }

    #[inline(always)]
    fn next_byte(&mut self) -> u8 {
        if self.input_pos == self.input_len {
            self.refill();
            if self.input_pos == self.input_len {
                self.input_ended = true;
                return 0;
            }
        }
        let b = self.input[self.input_pos];
        self.input_pos += 1;
        b
    }

    #[cold]
    fn refill(&mut self) {
        self.input_pos = 0;
        self.input_len = 0;
        if self.input_error.is_some() {
            return;
        }
        loop {
            match self.inner.read(&mut self.input) {
                Ok(n) => {
                    self.input_len = n;
                    return;
                }
                Err(e) if e.kind() == io::ErrorKind::Interrupted => {}
                Err(e) => {
                    self.input_error = Some(e);
                    return;
                }
            }
        }
    }

    fn check_input(&mut self) -> io::Result<()> {
        if let Some(e) = self.input_error.take() {
            // (kept failing from here on)
            self.input_error = Some(io::Error::new(e.kind(), "LZMA input failed earlier"));
            return Err(e);
        }
        if self.input_ended {
            return Err(io::Error::new(io::ErrorKind::UnexpectedEof, "LZMA stream ends too early"));
        }
        Ok(())
    }

    #[inline(always)]
    fn normalize(&mut self) {
        if self.range < TOP {
            self.range <<= 8;
            self.code = (self.code << 8) | self.next_byte() as u32;
        }
    }

    #[inline(always)]
    fn bit(&mut self, index: usize) -> u32 {
        self.normalize();
        let prob = &mut self.probs[index];
        let p = *prob as u32;
        let bound = (self.range >> 11) * p;
        if self.code < bound {
            self.range = bound;
            *prob = (p + ((2048 - p) >> 5)) as u16;
            0
        } else {
            self.range -= bound;
            self.code -= bound;
            *prob = (p - (p >> 5)) as u16;
            1
        }
    }

    #[inline(always)]
    fn direct_bits(&mut self, count: u32) -> u32 {
        let mut value = 0u32;
        for _ in 0..count {
            self.normalize();
            self.range >>= 1;
            let bit = if self.code >= self.range {
                self.code -= self.range;
                1
            } else {
                0
            };
            value = (value << 1) | bit;
        }
        value
    }

    #[inline(always)]
    fn bit_tree(&mut self, base: usize, bits: u32) -> u32 {
        let mut m = 1usize;
        for _ in 0..bits {
            m = (m << 1) | self.bit(base + m) as usize;
        }
        (m - (1usize << bits)) as u32
    }

    #[inline(always)]
    fn bit_tree_reverse(&mut self, base: usize, bits: u32) -> u32 {
        let mut m = 1usize;
        let mut value = 0u32;
        for i in 0..bits {
            let b = self.bit(base + m);
            m = (m << 1) | b as usize;
            value |= b << i;
        }
        value
    }

    fn length(&mut self, coder: usize, pos_state: usize) -> usize {
        if self.bit(coder) == 0 {
            return self.bit_tree(coder + 2 + pos_state * LEN_LOW, 3) as usize;
        }
        if self.bit(coder + 1) == 0 {
            return 8 + self.bit_tree(coder + 2 + (POS_STATES_MAX + pos_state) * LEN_LOW, 3) as usize;
        }
        16 + self.bit_tree(coder + 2 + 2 * POS_STATES_MAX * LEN_LOW, 8) as usize
    }

    #[inline(always)]
    fn byte_back(&self, distance: usize) -> u8 {
        // `distance` is 1 for the byte just written.
        let len = self.dict.len();
        let at = if self.dict_pos >= distance { self.dict_pos - distance } else { self.dict_pos + len - distance };
        self.dict[at]
    }

    #[inline(always)]
    fn put(&mut self, byte: u8) {
        self.dict[self.dict_pos] = byte;
        self.dict_pos += 1;
        if self.dict_pos == self.dict.len() {
            self.dict_pos = 0;
        }
        self.total += 1;
    }

    fn copy_match(&mut self, distance: usize, len: usize) {
        let ring = self.dict.len();
        let mut from =
            if self.dict_pos >= distance { self.dict_pos - distance } else { self.dict_pos + ring - distance };
        if distance >= len && from + len <= ring && self.dict_pos + len <= ring {
            // No wrap and no overlap: one block move.
            self.dict.copy_within(from..from + len, self.dict_pos);
            self.dict_pos += len;
            if self.dict_pos == ring {
                self.dict_pos = 0;
            }
            self.total += len as u64;
            return;
        }
        for _ in 0..len {
            let b = self.dict[from];
            from += 1;
            if from == ring {
                from = 0;
            }
            self.put(b);
        }
    }

    /// Decodes until about `BATCH` bytes wait to be handed out, or the stream ends.
    fn decode_batch(&mut self) -> io::Result<()> {
        while !self.finished && ((self.total - self.given) as usize) < BATCH {
            let pos_state = (self.total & self.pb_mask) as usize;
            if self.bit(IS_MATCH + (self.state << 4) + pos_state) == 0 {
                let prev = if self.total == 0 { 0 } else { self.byte_back(1) as usize };
                let lit_state = (((self.total & self.lp_mask) as usize) << self.lc) + (prev >> (8 - self.lc));
                let base = LITERAL + 0x300 * lit_state;
                let mut symbol = 1usize;
                if self.state >= 7 {
                    let mut match_byte = self.byte_back(self.reps[0] as usize + 1) as usize;
                    while symbol < 0x100 {
                        let match_bit = (match_byte >> 7) & 1;
                        match_byte <<= 1;
                        let b = self.bit(base + ((1 + match_bit) << 8) + symbol) as usize;
                        symbol = (symbol << 1) | b;
                        if match_bit != b {
                            break;
                        }
                    }
                }
                while symbol < 0x100 {
                    symbol = (symbol << 1) | self.bit(base + symbol) as usize;
                }
                self.put(symbol as u8);
                self.state = match self.state {
                    0..=3 => 0,
                    4..=9 => self.state - 3,
                    _ => self.state - 6,
                };
                continue;
            }
            let len;
            if self.bit(IS_REP + self.state) != 0 {
                if self.total == 0 {
                    return Err(bad("LZMA stream repeats a match before any byte"));
                }
                if self.bit(IS_REP_G0 + self.state) == 0 {
                    if self.bit(IS_REP0_LONG + (self.state << 4) + pos_state) == 0 {
                        self.state = if self.state < 7 { 9 } else { 11 };
                        self.check_distance(self.reps[0])?;
                        let b = self.byte_back(self.reps[0] as usize + 1);
                        self.put(b);
                        continue;
                    }
                } else {
                    let distance;
                    if self.bit(IS_REP_G1 + self.state) == 0 {
                        distance = self.reps[1];
                    } else {
                        if self.bit(IS_REP_G2 + self.state) == 0 {
                            distance = self.reps[2];
                        } else {
                            distance = self.reps[3];
                            self.reps[3] = self.reps[2];
                        }
                        self.reps[2] = self.reps[1];
                    }
                    self.reps[1] = self.reps[0];
                    self.reps[0] = distance;
                }
                len = self.length(REP_LEN_CODER, pos_state);
                self.state = if self.state < 7 { 8 } else { 11 };
            } else {
                self.reps[3] = self.reps[2];
                self.reps[2] = self.reps[1];
                self.reps[1] = self.reps[0];
                len = self.length(LEN_CODER, pos_state);
                self.state = if self.state < 7 { 7 } else { 10 };
                let slot = self.bit_tree(POS_SLOT + len.min(3) * 64, 6);
                if slot < 4 {
                    self.reps[0] = slot;
                } else {
                    let direct = (slot >> 1) - 1;
                    let mut distance = (2 | (slot & 1)) << direct;
                    if slot < END_POS_MODEL_INDEX {
                        distance += self.bit_tree_reverse(POS_SPECIAL + distance as usize - slot as usize, direct);
                    } else {
                        distance += self.direct_bits(direct - 4) << 4;
                        distance += self.bit_tree_reverse(ALIGN, 4);
                    }
                    self.reps[0] = distance;
                    if distance == 0xffff_ffff {
                        self.finished = true;
                        break;
                    }
                }
            }
            self.check_distance(self.reps[0])?;
            self.copy_match(self.reps[0] as usize + 1, len + MATCH_MIN_LEN);
        }
        self.check_input()
    }

    #[inline(always)]
    fn check_distance(&self, distance: u32) -> io::Result<()> {
        if distance as u64 >= self.total || distance as u64 >= self.dict_size {
            // (a truncated input feeds zeros, which can decode to anything: say that instead)
            if self.input_ended {
                return Err(io::Error::new(io::ErrorKind::UnexpectedEof, "LZMA stream ends too early"));
            }
            return Err(bad("LZMA match reaches before the start of the data"));
        }
        Ok(())
    }
}

impl<R: Read> Read for Lzma1Reader<R> {
    fn read(&mut self, out: &mut [u8]) -> io::Result<usize> {
        if out.is_empty() {
            return Ok(0);
        }
        if self.given == self.total {
            if self.finished {
                return Ok(0);
            }
            self.decode_batch()?;
        }
        let waiting = (self.total - self.given) as usize;
        let n = waiting.min(out.len());
        let ring = self.dict.len();
        let start = if self.dict_pos >= waiting { self.dict_pos - waiting } else { self.dict_pos + ring - waiting };
        let first = n.min(ring - start);
        out[..first].copy_from_slice(&self.dict[start..start + first]);
        out[first..n].copy_from_slice(&self.dict[..n - first]);
        self.given += n as u64;
        Ok(n)
    }
}

fn bad(text: &str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, text)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Forty made-up lines of text packed by liblzma (lc 3, lp 0, pb 2, 64 KiB dictionary, with the end marker),
    /// without the 8-byte size field of an .lzma file: the shape of a stream in an Inno Setup package.
    const PACKED: [u8; 95] = [
        0x5d, 0x00, 0x00, 0x01, 0x00, 0x00, 0x36, 0x1a, 0x4a, 0x1f, 0x08, 0xa0, 0x26, 0x56, 0x4e, 0x0d, 0x6c, 0xb8,
        0xa5, 0xed, 0x63, 0x9c, 0x8e, 0x7c, 0xdb, 0x4e, 0xf6, 0x9e, 0x4b, 0x78, 0x18, 0x56, 0x5c, 0xf7, 0x26, 0xeb,
        0xd4, 0xa3, 0x6e, 0x1c, 0x46, 0x10, 0x0c, 0x58, 0x6a, 0xe7, 0x41, 0x92, 0x1d, 0x7f, 0x58, 0x4b, 0xc5, 0x7f,
        0x5f, 0x6a, 0xfe, 0x1b, 0xf3, 0xa7, 0xc4, 0xba, 0x2c, 0xaf, 0xf3, 0xec, 0x95, 0xf0, 0x9a, 0x6f, 0xf3, 0x0a,
        0x11, 0xdc, 0x1c, 0xd8, 0x22, 0x78, 0xed, 0xfb, 0x42, 0xdb, 0xbb, 0x7d, 0xaa, 0x4d, 0x2d, 0x3f, 0xab, 0xff,
        0xff, 0xd6, 0xed, 0x8a, 0x00,
    ];

    fn plain() -> Vec<u8> {
        (0..40).flat_map(|i| format!("line {}: the quick brown fox jumps over the lazy dog
", i % 7).into_bytes()).collect()
    }

    #[test]
    fn decodes_a_small_stream() {
        let mut r = Lzma1Reader::new(&PACKED[..]).unwrap();
        let mut out = Vec::new();
        r.read_to_end(&mut out).unwrap();
        assert!(r.finished());
        assert_eq!(out, plain());
    }

    #[test]
    fn decodes_in_small_reads() {
        // One input byte per read, three output bytes per read.
        struct OneByte<'a>(&'a [u8]);
        impl Read for OneByte<'_> {
            fn read(&mut self, out: &mut [u8]) -> io::Result<usize> {
                let n = self.0.len().min(out.len()).min(1);
                out[..n].copy_from_slice(&self.0[..n]);
                self.0 = &self.0[n..];
                Ok(n)
            }
        }
        let mut r = Lzma1Reader::new(OneByte(&PACKED)).unwrap();
        let mut out = Vec::new();
        let mut piece = [0u8; 3];
        loop {
            let n = r.read(&mut piece).unwrap();
            if n == 0 {
                break;
            }
            out.extend_from_slice(&piece[..n]);
        }
        assert_eq!(out, plain());
    }

    #[test]
    fn cut_stream_is_an_error_not_short_data() {
        let mut r = Lzma1Reader::new(&PACKED[..60]).unwrap();
        let mut out = Vec::new();
        let e = r.read_to_end(&mut out).unwrap_err();
        assert_eq!(e.kind(), io::ErrorKind::UnexpectedEof);
    }

    #[test]
    fn rejects_bad_properties() {
        assert!(Lzma1Reader::new(&[0xff, 0, 0, 0x10, 0, 0, 0, 0, 0, 0][..]).is_err());
        assert!(Lzma1Reader::new(&[0x5d, 0, 0][..]).is_err());
    }

    #[test]
    fn truncated_stream_is_an_error() {
        // Properties and four of the five initial bytes.
        let r = Lzma1Reader::new(&[0x5d, 0, 0, 0x10, 0, 0, 1, 2, 3][..]);
        assert!(r.is_err());
    }
}
