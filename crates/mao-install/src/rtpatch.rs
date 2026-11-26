//! Applying a Pocket Soft RTPatch 5.21 patch file (`.RTP`), the format of Funcom's patches for the game's loose
//! files and, inside the resource-database patches, for single records.
//!
//! Read from the game's `PATCHW32.DLL` (".RTPatch Software Update System DLL version 5.21"), whose
//! `RTPatchApply32@12` the original patcher calls. The addresses below are that DLL's. Checked against Funcom's
//! twelve patches from 18.8.50 to 18.8.62 (`tests/patches.rs`): only what those patches use is handled, and a patch
//! that uses anything else is refused with [`Error::Unsupported`] rather than guessed at.
//!
//! # The file
//! Little-endian. Numbers marked *v* are variable-length ([`Reader::varint`], `FUN_10019260`); a string is a length
//! byte (0xff: a u16 follows), then that many bytes, the last a NUL (`FUN_10019440`).
//!
//! The header (`FUN_1000b420`, then `FUN_1000acd0`): `u16` 0x2a4b, `u16` version (0x209), `u16` flags, `u32` more
//! flags when flags has 0x8000, `u16` the entries' default flags, `u32` and `u32` disk space wanted, `u16` the default
//! attribute mask, 2 bytes, `u16` format flags, `u32` when the format flags have 4, 4 bytes, the backup folder's
//! name when flags has 1 (`PATCHESUNDO`), `u16` and that many folder names when flags has 0x200.
//!
//! Then entries (`FUN_10013c70`) until one of type 1. An entry: `u16` flags, the type in the top four bits (2 a new
//! file, 3 a file to delete, 4 a file to change); `u16` own flags when it has 2; a name when it has 4; a folder
//! number *v* (and one more *v* when the header's more-flags have 0x10000: long names) when its own flags have 0x80
//! or 0x40; ten bytes; and by type (`FUN_10008f10`) a `u16` of how to change, the count *v* of old files, the count
//! *v* of new files, `u32` the size of the entry's data unpacked and `u32` packed. Then each old and each new file:
//! 14 bytes of 8.3 name, `u16` attributes, `u32` size, `u16` DOS time, `u16` DOS date, ten bytes of checksum
//! ([`checksum`]), and with long names eight bytes of times and the long name. Then the data.
//!
//! The data is packed ([`unpack`]) unless the entry's flags have 1. For a new file it is the file. For a changed
//! file it is a list of commands (`FUN_10014790`, [`patch`]): copy a range of the old file, leave a hole, fill with
//! a pattern, add a value to bytes already written; the holes' bytes all come together with command 5.
//!
//! # What is not done
//! RTPatch keeps the old files in the backup folder with a script that puts them back; nothing is kept here (the
//! changes are all worked out and checked before the first file is written). Files' attributes are not set.

use std::collections::HashMap;
use std::fmt;
use std::io;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

#[derive(Debug)]
pub enum Error {
    /// The patch file is not what the format says.
    Format(String),
    /// The patch uses a part of RTPatch that Funcom's patches never used, so it was never checked here.
    Unsupported(String),
    /// A file to change is missing or is not the version the patch was made for.
    OldFile { path: String, missing: bool },
    /// A file came out with another checksum than the patch gives for it.
    Verify { path: String },
    Io(io::Error),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Format(what) => write!(f, "bad patch file: {what}"),
            Error::Unsupported(what) => write!(f, "the patch uses something this patcher does not know: {what}"),
            Error::OldFile { path, missing: true } => write!(f, "{path} is missing: the patch has nothing to change"),
            Error::OldFile { path, missing: false } => {
                write!(f, "{path} is not the version this patch changes (another size or checksum)")
            }
            Error::Verify { path } => write!(f, "{path} came out wrong after patching (checksum)"),
            Error::Io(e) => write!(f, "{e}"),
        }
    }
}

impl std::error::Error for Error {}

impl From<io::Error> for Error {
    fn from(e: io::Error) -> Error {
        Error::Io(e)
    }
}

pub type Result<T> = std::result::Result<T, Error>;

fn bad<T>(what: impl Into<String>) -> Result<T> {
    Err(Error::Format(what.into()))
}

fn unsupported<T>(what: impl Into<String>) -> Result<T> {
    Err(Error::Unsupported(what.into()))
}

/// Reads the patch file's fields.
pub struct Reader<'a> {
    d: &'a [u8],
    p: usize,
}

impl<'a> Reader<'a> {
    pub fn new(d: &'a [u8]) -> Reader<'a> {
        Reader { d, p: 0 }
    }

    pub fn position(&self) -> usize {
        self.p
    }

    pub fn raw(&mut self, n: usize) -> Result<&'a [u8]> {
        let end = self.p.checked_add(n).filter(|&end| end <= self.d.len());
        let Some(end) = end else { return bad(format!("it ends inside a field at {:#x}", self.p)) };
        let s = &self.d[self.p..end];
        self.p = end;
        Ok(s)
    }

    pub fn u8(&mut self) -> Result<u8> {
        Ok(self.raw(1)?[0])
    }

    pub fn u16(&mut self) -> Result<u16> {
        Ok(u16::from_le_bytes(self.raw(2)?.try_into().unwrap()))
    }

    pub fn u32(&mut self) -> Result<u32> {
        Ok(u32::from_le_bytes(self.raw(4)?.try_into().unwrap()))
    }

    /// A variable-length number (`FUN_10019260`): the first byte's top bit is the sign; then one bit for each byte
    /// that follows, from bit 6 down, ended by a clear bit; what is left of the first byte is the number's top part
    /// and the following bytes its low part, little-endian. A negative number comes back in two's complement.
    pub fn varint(&mut self) -> Result<u32> {
        let a = self.u8()?;
        let (mut more, mut flag, mut mask) = (0usize, 0x40u8, 0x3fu8);
        while flag & a != 0 {
            more += 1;
            mask >>= 1;
            flag >>= 1;
        }
        let mut bytes = [0u8; 4];
        if more == 0 {
            bytes[0] = a & mask;
        } else {
            if more > 4 {
                return bad(format!("a number of {more} bytes at {:#x}", self.p - 1));
            }
            bytes[..more].copy_from_slice(self.raw(more)?);
            if more != 4 {
                bytes[more] = a & mask;
            }
        }
        let v = u32::from_le_bytes(bytes);
        Ok(if a & 0x80 != 0 { v.wrapping_neg() } else { v })
    }

    /// A string (`FUN_10019440`, the one-byte-per-character form).
    pub fn string(&mut self) -> Result<String> {
        let mut n = self.u8()? as usize;
        if n == 0xff {
            n = self.u16()? as usize;
        }
        let s = self.raw(n)?;
        let s = s.split(|&b| b == 0).next().unwrap_or(&[]);
        Ok(s.iter().map(|&b| b as char).collect())
    }
}

/// RTPatch's ten-byte checksum of a file (`FUN_10019bd0`, and the masks of `FUN_10003510`): the length modulo 31
/// and modulo 30, then two words through which every byte is exclusive-ored while they turn left by eight bits, one
/// 31 bits wide and one 30 bits wide.
pub fn checksum(data: &[u8]) -> [u8; 10] {
    let (mut b, mut d) = (0u32, 0u32);
    for &x in data {
        b ^= x as u32;
        d ^= x as u32;
        b = ((b << 8) | (b >> 23)) & 0x7fff_ffff;
        d = ((d << 8) | (d >> 22)) & 0x3fff_ffff;
    }
    let mut out = [0u8; 10];
    out[0] = (data.len() % 31) as u8;
    out[1] = (data.len() % 30) as u8;
    out[2..6].copy_from_slice(&b.to_le_bytes());
    out[6..10].copy_from_slice(&d.to_le_bytes());
    out
}

/// The packed stream's bits, the top bit of each byte first.
struct Bits<'a> {
    d: &'a [u8],
    p: usize,
    left: u8,
}

impl Bits<'_> {
    fn bit(&mut self) -> Result<u32> {
        let Some(&byte) = self.d.get(self.p) else { return bad("the packed data ends early") };
        self.left -= 1;
        let b = (byte >> self.left) & 1;
        if self.left == 0 {
            self.left = 8;
            self.p += 1;
        }
        Ok(b as u32)
    }

    fn bits(&mut self, n: u32) -> Result<u32> {
        let mut v = 0;
        for _ in 0..n {
            v = v * 2 + self.bit()?;
        }
        Ok(v)
    }
}

/// One of the packed stream's three codes (`FUN_1001bf90`): a prefix code over `1 << bits` symbols and an escape,
/// that starts with the escape alone and changes as symbols are met. A symbol met for the first time is the escape's
/// code and then the symbol's own bits. The symbols are kept in `order`, the shortest codes first; `count[l]` of
/// them have codes of `l + 1` bits, from `order[first[l]]`. The code of `l + 1` bits whose value is `w0[l]` or more
/// is a symbol's (the `value - w0[l]`th of its length), and a smaller one goes on to more bits.
struct Code {
    bits: u32,
    max_len: usize,
    escape: u16,
    symbols: usize,
    lens: usize,
    count: Vec<u16>,
    first: Vec<i32>,
    order: Vec<u16>,
    freq: Vec<u16>,
    w0: Vec<i32>,
    /// Symbols still to read before the lengths are worked out again, and what that count starts from.
    countdown: u16,
    reload: u16,
    reload0: u16,
    /// Rebuilds still to do before the frequencies are halved, and what that count starts from.
    halve: u16,
    halve_reload: u16,
    halve0: u16,
}

impl Code {
    fn new(bits: u32, max_len: usize, rebuild: u16, halve: u16) -> Code {
        let n = 1usize << bits;
        let mut count = vec![0u16; max_len + 2];
        count[0] = 2;
        let mut first = vec![2i32; max_len + 2];
        first[0] = 0;
        let mut order = vec![n as u16 + 1; n + 3];
        order[0] = n as u16;
        let mut freq = vec![0x8000u16; n + 2];
        freq[n] = 0;
        freq[n + 1] = 0;
        let mut code = Code {
            bits,
            max_len,
            escape: n as u16,
            symbols: 1,
            lens: 1,
            count,
            first,
            order,
            freq,
            w0: vec![0; max_len + 1],
            countdown: rebuild,
            reload: rebuild,
            reload0: rebuild,
            halve,
            halve_reload: halve,
            halve0: halve,
        };
        code.limits();
        code
    }

    /// `FUN_1001ba60`: where each length's symbols begin among the code values.
    fn limits(&mut self) {
        let mut w = 1i32;
        for l in 0..self.max_len {
            w = 2 * w - self.count[l] as i32;
            self.w0[l] = w;
        }
    }

    /// The frequency of the symbol at a place in `order`.
    fn freq_at(&self, at: i32) -> Result<u16> {
        let symbol = usize::try_from(at).ok().and_then(|at| self.order.get(at));
        match symbol {
            Some(&symbol) => Ok(self.freq[symbol as usize]),
            None => bad("the packed data's code went wrong"),
        }
    }

    /// `FUN_1001ad30`: the next symbol.
    fn decode(&mut self, bits: &mut Bits) -> Result<u16> {
        let (mut value, mut len) = (0i32, 0usize);
        let limit = loop {
            value = value * 2 + bits.bit()? as i32;
            len += 1;
            if len > self.max_len {
                return bad("the packed data has a code that is too long");
            }
            if value >= self.w0[len - 1] {
                break self.w0[len - 1];
            }
        };
        let at = self.first[len - 1] + value - limit;
        let symbol = usize::try_from(at).ok().and_then(|at| self.order.get(at)).copied();
        let Some(mut symbol) = symbol else { return bad("the packed data's code went wrong") };
        // FUN_1001bf60
        let f = &mut self.freq[symbol as usize];
        *f = f.wrapping_add(1);
        self.countdown = self.countdown.wrapping_sub(1);
        if self.countdown == 0 {
            self.rebuild()?;
            self.limits();
        }
        if symbol == self.escape {
            symbol = bits.bits(self.bits)? as u16;
            self.add(symbol)?;
            self.limits();
        }
        Ok(symbol)
    }

    /// `FUN_1001bea0`: a symbol met for the first time gets the longest code, made by lengthening another.
    fn add(&mut self, symbol: u16) -> Result<()> {
        if self.symbols + 1 >= self.order.len() {
            return bad("the packed data names a symbol twice");
        }
        self.freq[symbol as usize] = 1;
        self.order[self.symbols] = symbol;
        self.symbols += 1;
        if self.symbols == 2 {
            return Ok(());
        }
        let mut l = self.lens - 1;
        if self.lens < self.max_len {
            self.lens += 1;
        } else {
            l -= 1;
            while self.count[l] == 0 {
                if l == 0 {
                    return bad("the packed data's code is full");
                }
                l -= 1;
            }
        }
        self.count[l] -= 1;
        self.count[l + 1] += 2;
        self.first[l + 1] -= 1;
        for k in l + 2..=self.max_len {
            self.first[k] += 1;
        }
        Ok(())
    }

    /// `FUN_1001bb10`: the symbols sorted by how often they came, and the lengths moved towards what suits that.
    fn rebuild(&mut self) -> Result<()> {
        let n = self.symbols;
        self.halve = self.halve.wrapping_sub(1);
        let halve = self.halve == 0;
        let mut most = 0u16;
        for i in 0..n {
            let f = &mut self.freq[self.order[i] as usize];
            if halve {
                *f >>= 1;
            }
            most = most.max(*f);
        }
        if most != 0 {
            // (a sort by the frequencies' bits from the top one down, as the original does it: the order it leaves
            // equal frequencies in decides the codes)
            let mut mask = 0x8000u16;
            while most & mask == 0 {
                mask = (mask >> 1) | 0x8000;
            }
            let mut i = 0;
            while i < n {
                let mut e = self.order[i];
                if self.freq[e as usize] & mask != 0 {
                    i += 1;
                    continue;
                }
                let (mut j, mut k) = (i + 1, i);
                if n <= j {
                    break;
                }
                while j < n {
                    let other = self.order[j];
                    if self.freq[other as usize] & mask != 0 {
                        self.order[k] = other;
                        self.order[j] = e;
                        k += 1;
                        e = self.order[k];
                    }
                    j += 1;
                }
                if k != i {
                    i = k - 1;
                }
                let next = mask >> 1;
                mask = next | 0x8000;
                if next & 1 != 0 {
                    break;
                }
            }
        }
        let mut changes = 0u32;
        let mut i = 0usize;
        let mut last = self.lens - 1;
        while i < self.lens {
            let c = self.count[i];
            if c == 0 {
                i += 1;
                continue;
            }
            let lengthen = c < 3
                || self.max_len - 1 == i
                || (self.freq_at(self.first[i])? as u32)
                    < self.freq_at(self.first[i + 1] - 1)? as u32 + self.freq_at(self.first[i + 1] - 2)? as u32;
            if lengthen {
                // The last symbol of this length gets a bit more, if further down two symbols that came more often
                // than it can each lose one.
                let mut rest = self.freq_at(self.first[i + 1] - 1)?;
                let mut j = i + 2;
                let mut found = false;
                while j < self.lens {
                    rest = rest.wrapping_sub(self.freq_at(self.first[j])?);
                    if self.count[j] > 1 && (rest & 0x8000 != 0 || rest < self.freq_at(self.first[j] + 1)?) {
                        found = true;
                        break;
                    }
                    j += 1;
                }
                if !found {
                    i += 1;
                    continue;
                }
                changes += 1;
                self.count[i] = c - 1;
                self.first[i + 1] -= 1;
                self.count[i + 1] += 2;
                for k in i + 2..j {
                    self.first[k] += 1;
                }
                self.first[j] += 2;
                self.count[j - 1] += 1;
                self.count[j] = self.count[j].wrapping_sub(2);
                if self.count[last] == 0 {
                    self.lens -= 1;
                    last = last.wrapping_sub(1);
                }
            } else {
                // Three symbols of this length: the first gets a bit less, the last two a bit more.
                if i == 0 || i + 1 > self.max_len {
                    return bad("the packed data's code went wrong");
                }
                changes += 1;
                self.count[i - 1] += 1;
                self.count[i] -= 3;
                self.count[i + 1] += 2;
                self.first[i] += 1;
                self.first[i + 1] -= 2;
                if last == i {
                    self.lens += 1;
                    last += 1;
                }
            }
            if self.lens == 0 || self.lens > self.max_len || last >= self.count.len() {
                return bad("the packed data's code went wrong");
            }
            i = 0;
        }
        if changes >= 0x10 {
            self.reload = self.reload0;
            self.halve_reload = self.halve0;
        } else if changes < 8 && self.halve_reload != 1 {
            self.reload <<= 1;
            self.halve >>= 1;
            self.halve_reload >>= 1;
        }
        self.countdown = self.reload;
        if self.halve == 0 {
            self.halve = self.halve_reload;
        }
        Ok(())
    }
}

/// The start of a packed stream.
const PACKED: u32 = 0xb59c;

/// Unpacks RTPatch's packed stream (`FUN_1001b430`, `FUN_1001ac10`, `FUN_1001af00`): bytes and copies of earlier
/// bytes, each with a leading bit that says which. A byte is a symbol of the first code. A copy is the low bits of
/// its distance as they are, the distance's high bits as a symbol of the third code, and the length as a symbol of
/// the second; it starts one byte further back than the distance says, and bytes from before the start are zero. A
/// distance of zero ends the stream. `size` is what the data must unpack to.
pub fn unpack(data: &[u8], size: usize) -> Result<Vec<u8>> {
    let mut bits = Bits { d: data, p: 0, left: 8 };
    if bits.bits(16)? != PACKED {
        return bad("the packed data does not start as it should");
    }
    let plain_bytes = bits.bits(8)? != 0;
    bits.bits(8)?;
    let rebuild = bits.bits(12)? as u16;
    let halve = bits.bits(12)? as u16;
    let low_bits = if bits.bits(4)? == 8 { 7 } else { 6 };
    if rebuild == 0 || halve == 0 {
        return bad("the packed data's settings are zero");
    }
    let mut bytes = Code::new(8, 0x10, rebuild, halve);
    let mut lengths = Code::new(6, 0xc, rebuild, halve);
    let mut distances = Code::new(6, 0xc, rebuild, halve);
    let mut out = Vec::with_capacity(size);
    while out.len() < size {
        if bits.bit()? == 0 {
            let byte = if plain_bytes { bits.bits(8)? as u8 } else { bytes.decode(&mut bits)? as u8 };
            out.push(byte);
            continue;
        }
        let low = bits.bits(low_bits)?;
        let distance = ((distances.decode(&mut bits)? as u32) << low_bits | low) as usize;
        if distance == 0 {
            break;
        }
        let n = lengths.decode(&mut bits)? as usize;
        if out.len() + n > size {
            return bad("the packed data unpacks to more than it says");
        }
        for _ in 0..n {
            let byte = if out.len() > distance { out[out.len() - distance - 1] } else { 0 };
            out.push(byte);
        }
    }
    if out.len() != size {
        return bad("the packed data unpacks to less than it says");
    }
    Ok(out)
}

/// Makes the new file from the old one and a changed file's commands (`FUN_10014790`).
pub fn patch(old: &[u8], commands: &[u8], new_size: usize) -> Result<Vec<u8>> {
    let mut r = Reader::new(commands);
    let mut out = vec![0u8; new_size];
    // Where the next bytes go, the stretches left for command 5 to fill, the ranges command 8 named, and where
    // the last value was added.
    let mut at = 0usize;
    let mut holes: Vec<(usize, usize)> = Vec::new();
    let mut ranges: Vec<(usize, usize)> = Vec::new();
    let mut added = 0u32;

    fn room(out: &[u8], at: usize, n: usize) -> Result<std::ops::Range<usize>> {
        match at.checked_add(n) {
            Some(end) if end <= out.len() => Ok(at..end),
            _ => bad("a command writes past the new file's end"),
        }
    }
    fn hole(out: &[u8], holes: &mut Vec<(usize, usize)>, at: &mut usize, gap: usize) -> Result<()> {
        if gap != 0 {
            room(out, *at, gap)?;
            holes.push((*at, gap));
            *at += gap;
        }
        Ok(())
    }
    fn copy(out: &mut [u8], old: &[u8], at: &mut usize, from: usize, n: usize) -> Result<()> {
        let to = room(out, *at, n)?;
        let Some(source) = from.checked_add(n).and_then(|end| old.get(from..end)) else {
            return bad("a command copies from past the old file's end");
        };
        out[to].copy_from_slice(source);
        *at += n;
        Ok(())
    }
    fn fill(out: &mut [u8], at: &mut usize, pattern: [u8; 4], n: usize) -> Result<()> {
        let to = room(out, *at, n)?;
        for (i, byte) in out[to].iter_mut().enumerate() {
            *byte = pattern[i & 3];
        }
        *at += n;
        Ok(())
    }
    fn add(out: &mut [u8], added: &mut u32, step: u32, value: &[u8]) -> Result<()> {
        *added = added.wrapping_add(step);
        let to = room(out, *added as usize, value.len())?;
        let mut carry = 0u16;
        for (byte, &v) in out[to].iter_mut().zip(value) {
            let sum = *byte as u16 + v as u16 + carry;
            *byte = sum as u8;
            carry = sum >> 8;
        }
        Ok(())
    }

    loop {
        let command = r.u8()?;
        // (the commands that come in a form with a hole before them: the hole's size first)
        let gap = match command {
            4 | 10 | 0xc | 0x14 | 0x15 | 0x16 => r.varint()? as usize,
            _ => 0,
        };
        match command {
            1 => break,
            2 => {
                if r.varint()? != 0 {
                    return unsupported("an entry that changes more than one file");
                }
                added = 0;
            }
            3 | 4 => {
                let (from, n) = (r.varint()? as usize, r.varint()? as usize);
                hole(&out, &mut holes, &mut at, gap)?;
                copy(&mut out, old, &mut at, from, n)?;
            }
            5 => {
                if at < new_size {
                    holes.push((at, new_size - at));
                }
                for (start, n) in holes.drain(..) {
                    out[start..start + n].copy_from_slice(r.raw(n)?);
                }
            }
            6 => {
                let step = r.varint()?;
                let value = r.u8()?;
                add(&mut out, &mut added, step, &[value])?;
            }
            7 | 0xe => {
                added = 0;
                let value = r.u8()?;
                for _ in 0..r.varint()? {
                    let step = r.varint()?;
                    add(&mut out, &mut added, step, &[value])?;
                }
            }
            8 => ranges.push((r.varint()? as usize, r.varint()? as usize)),
            9 | 10 => {
                let which = r.varint()? as usize;
                let Some(&(from, n)) = ranges.get(which) else { return bad("a command names a range never given") };
                hole(&out, &mut holes, &mut at, gap)?;
                copy(&mut out, old, &mut at, from, n)?;
            }
            0xb | 0xc => {
                let n = r.varint()? as usize;
                hole(&out, &mut holes, &mut at, gap)?;
                fill(&mut out, &mut at, [0; 4], n)?;
            }
            0xd => {
                added = 0;
                for _ in 0..r.varint()? {
                    let step = r.varint()?;
                    let value = r.u8()?;
                    add(&mut out, &mut added, step, &[value])?;
                }
            }
            0xf | 0x10 => {
                added = 0;
                let value = r.raw(if command == 0xf { 2 } else { 4 })?;
                for _ in 0..r.varint()? {
                    let step = r.varint()?;
                    add(&mut out, &mut added, step, value)?;
                }
            }
            0x11 | 0x14 => {
                let b = r.u8()?;
                let n = r.varint()? as usize;
                hole(&out, &mut holes, &mut at, gap)?;
                fill(&mut out, &mut at, [b; 4], n)?;
            }
            0x12 | 0x15 => {
                let b = r.raw(2)?;
                let n = r.varint()? as usize;
                hole(&out, &mut holes, &mut at, gap)?;
                fill(&mut out, &mut at, [b[0], b[1], b[0], b[1]], n)?;
            }
            0x13 | 0x16 => {
                let b: [u8; 4] = r.raw(4)?.try_into().unwrap();
                let n = r.varint()? as usize;
                hole(&out, &mut holes, &mut at, gap)?;
                fill(&mut out, &mut at, b, n)?;
            }
            other => return bad(format!("an unknown command {other:#x}")),
        }
    }
    Ok(out)
}

/// A file as an entry describes it.
#[derive(Clone, Debug)]
pub struct FileInfo {
    /// The file's name: the long one when the patch has long names, else the 8.3 one.
    pub name: String,
    pub size: u32,
    pub attributes: u16,
    /// The file's time, as DOS keeps it: seconds / 2, minutes and hours in `time`; day, month and year - 1980 in
    /// `date`.
    pub time: u16,
    pub date: u16,
    pub checksum: [u8; 10],
}

impl FileInfo {
    fn is(&self, data: &[u8]) -> bool {
        data.len() == self.size as usize && checksum(data) == self.checksum
    }

    /// The file's time taken as UTC (RTPatch sets it as local time; which zone Funcom built in is not in the file).
    pub fn modified(&self) -> Option<SystemTime> {
        let (day, month, year) = (self.date & 31, (self.date >> 5) & 15, 1980 + (self.date >> 9) as i64);
        if day == 0 || month == 0 || month > 12 {
            return None;
        }
        // (days from 1970-01-01, the civil-calendar way)
        let (y, m) = if month <= 2 { (year - 1, month as i64 + 9) } else { (year, month as i64 - 3) };
        let era = y.div_euclid(400);
        let yoe = y.rem_euclid(400);
        let doy = (153 * m + 2) / 5 + day as i64 - 1;
        let days = era * 146097 + yoe * 365 + yoe / 4 - yoe / 100 + doy - 719468;
        let seconds = (self.time & 31) as i64 * 2 + ((self.time >> 5) & 63) as i64 * 60 + (self.time >> 11) as i64 * 3600;
        u64::try_from(days * 86400 + seconds).ok().map(|s| SystemTime::UNIX_EPOCH + Duration::from_secs(s))
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    /// A file the patch brings whole.
    Add,
    /// A file the patch takes away.
    Delete,
    /// A file the patch changes.
    Modify,
}

/// One entry of a patch file.
#[derive(Clone, Debug)]
pub struct Entry<'a> {
    pub kind: Kind,
    /// The folder the files are in, with `\` between its parts; empty for the folder the patch is applied to.
    pub folder: String,
    pub old: Vec<FileInfo>,
    pub new: Vec<FileInfo>,
    /// A missing or different old file is passed over rather than an error (the entry's own flags' 8).
    pub optional: bool,
    packed: bool,
    plain: usize,
    data: &'a [u8],
}

impl Entry<'_> {
    /// A file's path from the folder the patch is applied to, with `\` between its parts as the patch has them.
    pub fn path(&self, file: &FileInfo) -> String {
        if self.folder.is_empty() { file.name.clone() } else { format!("{}\\{}", self.folder, file.name) }
    }

    /// The entry's data, unpacked: a new file's bytes, or a changed file's commands.
    pub fn data(&self) -> Result<Vec<u8>> {
        if self.packed {
            unpack(self.data, self.plain)
        } else if self.data.len() == self.plain {
            Ok(self.data.to_vec())
        } else {
            bad("an entry's data is not the size it says")
        }
    }
}

/// A patch file, read.
#[derive(Clone, Debug)]
pub struct Patch<'a> {
    pub version: u16,
    /// The folder RTPatch would keep the old files in (`PATCHESUNDO`).
    pub backup_folder: Option<String>,
    /// The folders the patch knows, by number.
    pub folders: Vec<String>,
    pub entries: Vec<Entry<'a>>,
}

const MAGIC: u16 = 0x2a4b;

impl<'a> Patch<'a> {
    pub fn parse(data: &'a [u8]) -> Result<Patch<'a>> {
        let mut r = Reader::new(data);
        if r.u16()? != MAGIC {
            return bad("it does not start as an RTPatch file does");
        }
        let version = r.u16()?;
        if version > 0x209 {
            return unsupported(format!("a patch file of version {version:#x}"));
        }
        if version == 0xd2 {
            return unsupported("a patch file of the old version 0xd2");
        }
        let flags = r.u16()?;
        let more = if flags & 0x8000 != 0 { r.u32()? } else { 0 };
        if more & 7 != 0 {
            return unsupported("a patch file built for another system");
        }
        let long_names = more & 0x1_0000 != 0;
        let default_flags = r.u16()?;
        r.raw(4 + 4 + 2 + 2)?;
        let format = r.u16()?;
        if format & 8 != 0 {
            return unsupported("names in two-byte characters");
        }
        if format & 4 != 0 {
            r.u32()?;
        }
        r.raw(4)?;
        let backup_folder = if flags & 1 != 0 { Some(r.string()?) } else { None };
        if format & 0x10 != 0 {
            return unsupported("a patch file with a registry part");
        }
        if format & 0x20 != 0 {
            r.string()?;
        }
        if format & 0x40 != 0 {
            r.string()?;
        }
        if flags & 0x800 != 0 {
            return unsupported("a patch file with a history part");
        }
        let mut folders = Vec::new();
        if flags & 0x200 != 0 {
            for _ in 0..r.u16()? {
                folders.push(r.string()?);
            }
        }
        if flags & 2 != 0 {
            for _ in 0..r.u16()? {
                r.string()?;
            }
        }
        if flags & 8 != 0 {
            return unsupported("a patch file with a password");
        }

        let mut entries = Vec::new();
        loop {
            let at = r.position();
            let entry_flags = r.u16()?;
            let kind = match entry_flags >> 12 {
                1 => break,
                2 => Kind::Add,
                3 => Kind::Delete,
                4 => Kind::Modify,
                other => return unsupported(format!("an entry of type {other} at {at:#x}")),
            };
            let own = if entry_flags & 2 != 0 { r.u16()? } else { default_flags };
            if entry_flags & 4 != 0 {
                r.string()?;
            }
            let mut folder = String::new();
            match own & 0xc0 {
                0 => {}
                0x80 => {
                    let number = r.varint()? as usize;
                    if long_names {
                        r.varint()?;
                    }
                    let Some(name) = folders.get(number) else { return bad(format!("no folder {number} (entry at {at:#x})")) };
                    folder = name.trim_matches('\\').to_owned();
                }
                _ => return unsupported("an entry for a folder found on the search path"),
            }
            for (bit, what) in [(0x80, "its own old folder"), (0x200, "two names"), (8, "parts of the file left out of the checksum")] {
                if entry_flags & bit != 0 {
                    return unsupported(format!("an entry with {what} at {at:#x}"));
                }
            }
            if entry_flags & 0x100 != 0 {
                r.u16()?;
            }
            r.raw(10)?;
            let mut how = 1;
            let (mut old_count, mut new_count) = (0, 0);
            let (mut plain, mut packed_size) = (0, 0);
            if kind == Kind::Modify {
                how = r.u16()?;
            }
            if kind != Kind::Add {
                old_count = r.varint()? as usize;
            }
            if kind != Kind::Delete {
                new_count = r.varint()? as usize;
                plain = r.u32()? as usize;
                packed_size = r.u32()? as usize;
            }
            if kind == Kind::Modify && (how & 1 == 0 || how & 0x10 != 0) {
                return unsupported(format!("a way of changing a file ({how:#x}) at {at:#x}"));
            }
            let mut lists = [Vec::new(), Vec::new()];
            for (list, count) in lists.iter_mut().zip([old_count, new_count]) {
                for _ in 0..count {
                    let b = r.raw(0x18)?;
                    let word = |o: usize| u16::from_le_bytes([b[o], b[o + 1]]);
                    let mut file = FileInfo {
                        name: b[..14].split(|&c| c == 0).next().unwrap_or(&[]).iter().map(|&c| c as char).collect(),
                        attributes: word(14),
                        size: u32::from_le_bytes(b[16..20].try_into().unwrap()),
                        time: word(20),
                        date: word(22),
                        checksum: r.raw(10)?.try_into().unwrap(),
                    };
                    if long_names {
                        r.raw(8)?;
                        file.name = r.string()?;
                    }
                    if file.name.is_empty() || file.name.contains(['\\', '/', ':']) || file.name == ".." {
                        return bad(format!("a file name that is no name (entry at {at:#x})"));
                    }
                    list.push(file);
                }
            }
            if folder.split('\\').any(|part| part == ".." || part.contains(':')) || folder.starts_with('/') {
                return bad(format!("a folder outside the one being patched (entry at {at:#x})"));
            }
            let [old, new] = lists;
            entries.push(Entry {
                kind,
                folder,
                old,
                new,
                optional: own & 8 != 0,
                packed: entry_flags & 1 == 0,
                plain,
                data: r.raw(packed_size)?,
            });
        }
        Ok(Patch { version, backup_folder, folders, entries })
    }
}

/// The files a patch is applied to. A path is as the patch gives it: from the folder being patched, with `\`
/// between its parts, in whatever case the patch's maker had it.
pub trait Tree {
    /// The file's bytes, or `None` when there is no such file.
    fn read(&mut self, path: &str) -> io::Result<Option<Vec<u8>>>;
    fn write(&mut self, path: &str, data: &[u8], modified: Option<SystemTime>) -> io::Result<()>;
    fn remove(&mut self, path: &str) -> io::Result<()>;
}

/// A folder on the disk. Names are matched without regard to case, as Windows matches them, so a patch made on
/// Windows finds its files on a file system that tells cases apart.
pub struct Folder {
    root: PathBuf,
}

impl Folder {
    pub fn new(root: impl Into<PathBuf>) -> Folder {
        Folder { root: root.into() }
    }

    /// The path on the disk: each part as written when it is there, else the folder's entry of the same name in
    /// another case, else as written.
    pub fn locate(&self, path: &str) -> PathBuf {
        let mut at = self.root.clone();
        for part in path.split(['\\', '/']).filter(|part| !part.is_empty() && *part != ".") {
            let as_written = at.join(part);
            if as_written.exists() {
                at = as_written;
                continue;
            }
            let other = std::fs::read_dir(&at).ok().and_then(|entries| {
                entries.flatten().map(|e| e.file_name()).find(|name| name.to_str().is_some_and(|name| name.eq_ignore_ascii_case(part)))
            });
            at = match other {
                Some(name) => at.join(name),
                None => as_written,
            };
        }
        at
    }
}

/// Writes a file whole or not at all: beside its place first, then moved over it.
pub fn write_file(path: &Path, data: &[u8], modified: Option<SystemTime>) -> io::Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let mut beside = path.as_os_str().to_owned();
    beside.push(".mao-new");
    let beside = PathBuf::from(beside);
    let file = std::fs::File::create(&beside)?;
    io::Write::write_all(&mut &file, data)?;
    if let Some(modified) = modified {
        // (a file system that keeps no times is no reason to fail)
        let _ = file.set_modified(modified);
    }
    drop(file);
    std::fs::rename(&beside, path)
}

impl Tree for Folder {
    fn read(&mut self, path: &str) -> io::Result<Option<Vec<u8>>> {
        match std::fs::read(self.locate(path)) {
            Ok(data) => Ok(Some(data)),
            Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(None),
            Err(e) => Err(e),
        }
    }

    fn write(&mut self, path: &str, data: &[u8], modified: Option<SystemTime>) -> io::Result<()> {
        write_file(&self.locate(path), data, modified)
    }

    fn remove(&mut self, path: &str) -> io::Result<()> {
        match std::fs::remove_file(self.locate(path)) {
            Err(e) if e.kind() != io::ErrorKind::NotFound => Err(e),
            _ => Ok(()),
        }
    }
}

/// Files in memory, their names matched without regard to case.
#[derive(Default)]
pub struct Memory {
    pub files: HashMap<String, Vec<u8>>,
}

impl Tree for Memory {
    fn read(&mut self, path: &str) -> io::Result<Option<Vec<u8>>> {
        Ok(self.files.get(&path.to_ascii_lowercase()).cloned())
    }

    fn write(&mut self, path: &str, data: &[u8], _modified: Option<SystemTime>) -> io::Result<()> {
        self.files.insert(path.to_ascii_lowercase(), data.to_vec());
        Ok(())
    }

    fn remove(&mut self, path: &str) -> io::Result<()> {
        self.files.remove(&path.to_ascii_lowercase());
        Ok(())
    }
}

/// What applying a patch did, by path.
#[derive(Clone, Debug, Default)]
pub struct Report {
    pub added: Vec<String>,
    pub changed: Vec<String>,
    pub deleted: Vec<String>,
    /// Files that were already what the patch makes of them.
    pub up_to_date: Vec<String>,
    /// Old files that were missing or different where the patch says that does not matter.
    pub passed_over: Vec<String>,
}

enum Change {
    Write(String, Vec<u8>, Option<SystemTime>),
    Remove(String),
}

/// Applies a patch file to a tree of files. Every old file is checked against the patch's checksum for it and
/// every new file against its own before anything is written, so a patch that does not fit leaves the files as they
/// were. A file that is already what the patch makes of it is left alone, so a patch can be applied again.
pub fn apply(patch: &[u8], tree: &mut dyn Tree) -> Result<Report> {
    let patch = Patch::parse(patch)?;
    let mut report = Report::default();
    let mut changes = Vec::new();
    for entry in &patch.entries {
        match entry.kind {
            Kind::Add => {
                let data = entry.data()?;
                let mut rest = &data[..];
                for file in &entry.new {
                    let path = entry.path(file);
                    let Some((bytes, after)) = rest.split_at_checked(file.size as usize) else {
                        return bad(format!("{path}: the patch does not hold the whole file"));
                    };
                    rest = after;
                    if !file.is(bytes) {
                        return Err(Error::Verify { path });
                    }
                    if tree.read(&path)?.is_some_and(|there| file.is(&there)) {
                        report.up_to_date.push(path);
                        continue;
                    }
                    changes.push(Change::Write(path.clone(), bytes.to_vec(), file.modified()));
                    report.added.push(path);
                }
            }
            Kind::Delete => {
                for file in &entry.old {
                    let path = entry.path(file);
                    match tree.read(&path)? {
                        None => report.up_to_date.push(path),
                        Some(there) if file.is(&there) => {
                            changes.push(Change::Remove(path.clone()));
                            report.deleted.push(path);
                        }
                        Some(_) if entry.optional => report.passed_over.push(path),
                        Some(_) => return Err(Error::OldFile { path, missing: false }),
                    }
                }
            }
            Kind::Modify => {
                let ([old], [new]) = (&entry.old[..], &entry.new[..]) else {
                    return unsupported("an entry that changes more than one file");
                };
                let (old_path, new_path) = (entry.path(old), entry.path(new));
                if tree.read(&new_path)?.is_some_and(|there| new.is(&there)) {
                    report.up_to_date.push(new_path);
                    continue;
                }
                let there = tree.read(&old_path)?;
                let old_data = match there {
                    Some(there) if old.is(&there) => there,
                    _ if entry.optional => {
                        report.passed_over.push(old_path);
                        continue;
                    }
                    there => return Err(Error::OldFile { path: old_path, missing: there.is_none() }),
                };
                let made = patch_file(entry, &old_data, new)?;
                if !new.is(&made) {
                    return Err(Error::Verify { path: new_path });
                }
                if !old_path.eq_ignore_ascii_case(&new_path) {
                    changes.push(Change::Remove(old_path));
                }
                changes.push(Change::Write(new_path.clone(), made, new.modified()));
                report.changed.push(new_path);
            }
        }
    }
    for change in changes {
        match change {
            Change::Write(path, data, modified) => tree.write(&path, &data, modified)?,
            Change::Remove(path) => tree.remove(&path)?,
        }
    }
    Ok(report)
}

fn patch_file(entry: &Entry, old: &[u8], new: &FileInfo) -> Result<Vec<u8>> {
    patch(old, &entry.data()?, new.size as usize)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn numbers() {
        let read = |bytes: &[u8]| Reader::new(bytes).varint().unwrap();
        assert_eq!(read(&[0x00]), 0);
        assert_eq!(read(&[0x3f]), 0x3f);
        assert_eq!(read(&[0x41, 0x00]), 0x100);
        assert_eq!(read(&[0x40, 0x4e]), 0x4e);
        assert_eq!(read(&[0x61, 0x00, 0x76]), 0x1_7600);
        assert_eq!(read(&[0x70, 0x01, 0x02, 0x03]), 0x03_0201);
        assert_eq!(read(&[0x78, 0x01, 0x02, 0x03, 0x04]), 0x0403_0201);
        assert_eq!(read(&[0x81]), 1u32.wrapping_neg());
        assert_eq!(read(&[0xc1, 0x00]), 0x100u32.wrapping_neg());
    }

    #[test]
    fn strings() {
        let mut r = Reader::new(&[4, b'a', b'.', b'b', 0, 0xff, 3, 0, b'x', b'y', 0]);
        assert_eq!(r.string().unwrap(), "a.b");
        assert_eq!(r.string().unwrap(), "xy");
        assert!(r.string().is_err());
    }

    #[test]
    fn checksums() {
        // (the values the prototype that matched Funcom's patches gives)
        assert_eq!(checksum(b""), [0, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
        assert_eq!(checksum(b"a"), [1, 1, 0, 0x61, 0, 0, 0, 0x61, 0, 0]);
        let data: Vec<u8> = (0..1000u32).map(|i| (i * 7 + i / 3) as u8).collect();
        assert_eq!(checksum(&data)[..2], [(1000 % 31) as u8, (1000 % 30) as u8]);
        assert_eq!(checksum(&data)[5] & 0x80, 0);
        assert_eq!(checksum(&data)[9] & 0xc0, 0);
    }

    #[test]
    fn commands() {
        let old = b"0123456789abcdef";
        // copy 4 from 2; a hole of 3 then copy 2 from 10; four bytes of 'z'; the rest a hole; the holes' bytes;
        // add 1 to the byte at 1 and to the one two after; end
        let commands = [
            2, 0, 3, 2, 4, 4, 3, 10, 2, 0x11, b'z', 4, 5, b'X', b'Y', b'Z', b'!', b'?', 0xd, 2, 1, 1, 2, 1, 1,
        ];
        assert_eq!(patch(old, &commands, 15).unwrap(), b"2446XYZabzzzz!?");
        // (a copy from past the old file's end, and a write past the new one's)
        assert!(patch(old, &[2, 0, 3, 15, 2, 1], 2).is_err());
        assert!(patch(old, &[2, 0, 3, 0, 4, 1], 2).is_err());
    }
}
