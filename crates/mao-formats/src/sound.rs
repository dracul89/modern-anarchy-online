//! The sound system's data (SandyInterface.dll: the `sandyinterface` drop-in's `sandy.rs`, `source.rs` and `ids.rs`).
//!
//! - Sound IDs: a source is named by the hash of its name (`SandyInterface_t::CreateSoundID`, FUN_10008213), e.g. Gamecode's
//!   `GetSoundID("SM_Sandy_Game_Feet_Man_Walk")`.
//! - Source files (`cd_image/sound/SourceFiles/*.sbf`, `ParseSourceFile`): version 11, a count, then that many records of
//!   0xe4 bytes, each followed by its sample's file name (when it has one: a length and the bytes rotated right by 3) and
//!   its children's IDs.
//! - The samples (`SE_Create2DSoundFromFile`): RIFF waves (PCM, IMA ADPCM, MP3 in a wave) and Ogg Vorbis, read by Miles;
//!   here the waves are decoded and the compressed streams handed back for a decoder.

/// A sound's ID from its name (FUN_10008213): the first 255 bytes, lower-cased, taken four at a time as big-endian words,
/// each rotated left by its index and XORed together; a last partial word keeps its bytes in their places.
pub fn sound_id(name: &str) -> u32 {
    let bytes: Vec<u32> = name.bytes().take(0xff).map(|b| b.to_ascii_lowercase() as u32).collect();
    let mut h = 0u32;
    for (i, w) in bytes.chunks(4).enumerate() {
        let b = |k: usize| w.get(k).copied().unwrap_or(0);
        let word = (b(0) << 24) | (b(1) << 16) | (b(2) << 8) | b(3);
        h ^= word.rotate_left(i as u32 & 31);
    }
    h
}

/// A source's record, as `ParseSourceFile` reads it into the arguments of FUN_100111b0.
#[derive(Clone, Debug, PartialEq)]
pub struct Record {
    pub id: u32,
    /// Listed among all the sources (flag 1): the ones stopped and released together.
    pub listed: bool,
    pub looping: bool,
    /// Plays every child.
    pub all: bool,
    /// Plays one child at random.
    pub random: bool,
    pub has_file: bool,
    pub u16d: bool,
    /// Plays the children in turn.
    pub sequential: bool,
    /// The children play on timers.
    pub timed: bool,
    /// The chance in a second (100: always).
    pub chance: u8,
    /// Playing stops the sources of its group (`stop_group`).
    pub stop_others: bool,
    pub volume_min: f32,
    pub volume_max: f32,
    pub group: u8,
    pub stop_group: u8,
    /// 0 to 3 (lower plays over higher).
    pub priority: u8,
    pub alert_radius: f32,
    /// The delay passed on to the children.
    pub pass_delay: bool,
    pub doppler: bool,
    /// Seconds.
    pub interval_min: f32,
    pub interval_max: f32,
    pub length: f32,
    pub variants: [u32; 46],
    pub loop_min: f32,
    pub loop_max: f32,
    pub min_distance: f32,
    pub max_distance: f32,
    pub noise_radius: f32,
    pub fade_in: f32,
    /// The sample's file under `cd_image/sound`.
    pub file: String,
    pub children: Vec<u32>,
}

/// A source file's records (None when it isn't version 11 or is cut short).
pub fn parse_bank(data: &[u8]) -> Option<Vec<Record>> {
    let u32_at = |at: usize| Some(u32::from_le_bytes(data.get(at..at + 4)?.try_into().ok()?));
    let f32_at = |at: usize| u32_at(at).map(f32::from_bits);
    let u16_at = |at: usize| Some(u16::from_le_bytes(data.get(at..at + 2)?.try_into().ok()?));
    if u32_at(0)? != 0xb {
        return None;
    }
    let count = u32_at(4)?;
    let mut at = 8;
    let mut records = Vec::new();
    for _ in 0..count {
        let r = data.get(at..at + 0xe4)?;
        let (flags, chance, volume_min, volume_max, group, stop_group, priority, n) = (r[0], r[1], r[2], r[3], r[4], r[5], r[6], r[7]);
        let mut variants = [0u32; 46];
        for (k, v) in variants.iter_mut().enumerate() {
            *v = u32_at(at + 20 + 4 * k)?;
        }
        let floats = at + 20 + 46 * 4;
        // (the divisions are the original's, in doubles stored as floats)
        let tenths = |v: u16| (v as f64 / 100.0) as f32;
        let mut record = Record {
            id: u32_at(at + 16)?,
            listed: flags & 1 != 0,
            looping: flags & 2 != 0,
            all: flags & 4 != 0,
            random: flags & 8 != 0,
            has_file: flags & 0x10 != 0,
            u16d: flags & 0x20 != 0,
            sequential: flags & 0x40 != 0,
            timed: flags & 0x80 != 0,
            chance: chance & 0x7f,
            stop_others: chance & 0x80 != 0,
            volume_min: (volume_min as f64 / 255.0) as f32,
            volume_max: (volume_max as f64 / 255.0) as f32,
            group,
            stop_group,
            priority: (priority >> 5) & 3,
            alert_radius: ((priority & 0x1f) as f64 / 10.0) as f32,
            pass_delay: priority & 0x80 != 0,
            doppler: n & 0x20 != 0,
            interval_min: tenths(u16_at(at + 8)?),
            interval_max: tenths(u16_at(at + 10)?),
            length: tenths(u16_at(at + 12)?),
            variants,
            loop_min: f32_at(floats)?,
            loop_max: f32_at(floats + 4)?,
            min_distance: f32_at(floats + 8)?,
            max_distance: f32_at(floats + 12)?,
            noise_radius: f32_at(floats + 16)?,
            fade_in: f32_at(floats + 20)?,
            file: String::new(),
            children: Vec::new(),
        };
        at += 0xe4;
        if record.has_file {
            let size = u32_at(at)? as usize;
            at += 4;
            let name = data.get(at..at + size)?;
            at += size;
            let name: Vec<u8> = name.iter().map(|c| c.rotate_left(3)).collect();
            let end = name.iter().position(|&b| b == 0).unwrap_or(name.len()).min(0x107);
            record.file = name[..end].iter().map(|&b| b as char).collect();
        }
        for _ in 0..n & 0x1f {
            record.children.push(u32_at(at)?);
            at += 4;
        }
        records.push(record);
    }
    Some(records)
}

/// A sample file's sound.
#[derive(Debug, PartialEq)]
pub enum Sample<'a> {
    /// Decoded: the rate, the channels and the interleaved 16-bit samples.
    Pcm { rate: u32, channels: u16, samples: Vec<i16> },
    /// An MP3 stream (a wave of format 0x55's data, as Miles's `.mp3` decoder gets it).
    Mp3(&'a [u8]),
    /// An Ogg Vorbis stream (some `.wav`s of the music are).
    Ogg(&'a [u8]),
}

/// A sample file read as Miles reads it (`AIL_file_type`: waves of PCM and IMA ADPCM, MP3 and Ogg); None for others.
pub fn read_sample(data: &[u8]) -> Option<Sample<'_>> {
    if data.starts_with(b"OggS") {
        return Some(Sample::Ogg(data));
    }
    if data.starts_with(b"ID3") || (data.len() > 1 && data[0] == 0xff && data[1] & 0xe0 == 0xe0) {
        return Some(Sample::Mp3(data));
    }
    if data.get(..4)? != b"RIFF" || data.get(8..12)? != b"WAVE" {
        return None;
    }
    let mut fmt: Option<&[u8]> = None;
    let mut body: Option<&[u8]> = None;
    let mut at = 12;
    while at + 8 <= data.len() {
        let size = u32::from_le_bytes(data[at + 4..at + 8].try_into().ok()?) as usize;
        let chunk = &data[at + 8..(at + 8 + size).min(data.len())];
        match &data[at..at + 4] {
            b"fmt " => fmt = Some(chunk),
            b"data" => body = Some(chunk),
            _ => {}
        }
        at += 8 + size + (size & 1);
    }
    let (fmt, body) = (fmt?, body?);
    let half = |k: usize| Some(u16::from_le_bytes(fmt.get(k..k + 2)?.try_into().ok()?));
    let (format, channels, rate) = (half(0)?, half(2)?, u32::from_le_bytes(fmt.get(4..8)?.try_into().ok()?));
    let (align, bits) = (half(12)?, half(14)?);
    if channels == 0 || rate == 0 {
        return None;
    }
    match format {
        1 if bits == 16 => Some(Sample::Pcm { rate, channels, samples: body.chunks_exact(2).map(|b| i16::from_le_bytes([b[0], b[1]])).collect() }),
        1 if bits == 8 => Some(Sample::Pcm { rate, channels, samples: body.iter().map(|&b| ((b as i16) - 128) << 8).collect() }),
        0x11 => {
            let per_block = half(18).map(usize::from).unwrap_or(((align as usize - 4 * channels as usize) * 2) / channels as usize + 1);
            Some(Sample::Pcm { rate, channels, samples: ima_adpcm(body, channels as usize, align as usize, per_block) })
        }
        0x55 => Some(Sample::Mp3(body)),
        _ => None,
    }
}

const STEPS: [i32; 89] = [
    7, 8, 9, 10, 11, 12, 13, 14, 16, 17, 19, 21, 23, 25, 28, 31, 34, 37, 41, 45, 50, 55, 60, 66, 73, 80, 88, 97, 107, 118, 130, 143, 157, 173, 190, 209, 230,
    253, 279, 307, 337, 371, 408, 449, 494, 544, 598, 658, 724, 796, 876, 963, 1060, 1166, 1282, 1411, 1552, 1707, 1878, 2066, 2272, 2499, 2749, 3024, 3327,
    3660, 4026, 4428, 4871, 5358, 5894, 6484, 7132, 7845, 8630, 9493, 10442, 11487, 12635, 13899, 15289, 16818, 18500, 20350, 22385, 24623, 27086, 29794,
    32767,
];
const INDEX: [i32; 8] = [-1, -1, -1, -1, 2, 4, 6, 8];

/// IMA ADPCM in Microsoft's wave blocks: per channel a header (the first sample and the step index), then groups of four
/// bytes per channel in turn, eight samples each, low nibble first.
fn ima_adpcm(data: &[u8], channels: usize, align: usize, per_block: usize) -> Vec<i16> {
    let mut out = Vec::new();
    if align < 4 * channels {
        return out;
    }
    for block in data.chunks(align) {
        if block.len() < 4 * channels {
            break;
        }
        let mut state: Vec<(i32, i32)> = (0..channels).map(|c| (i16::from_le_bytes([block[4 * c], block[4 * c + 1]]) as i32, (block[4 * c + 2] as i32).clamp(0, 88))).collect();
        let mut samples: Vec<Vec<i16>> = state.iter().map(|&(p, _)| vec![p as i16]).collect();
        let body = &block[4 * channels..];
        for group in body.chunks(4 * channels) {
            for (c, bytes) in group.chunks(4).enumerate() {
                for &byte in bytes {
                    for nibble in [byte & 0xf, byte >> 4] {
                        let (predictor, index) = &mut state[c];
                        let step = STEPS[*index as usize];
                        let mut diff = step >> 3;
                        if nibble & 4 != 0 {
                            diff += step;
                        }
                        if nibble & 2 != 0 {
                            diff += step >> 1;
                        }
                        if nibble & 1 != 0 {
                            diff += step >> 2;
                        }
                        *predictor = if nibble & 8 != 0 { *predictor - diff } else { *predictor + diff }.clamp(-32768, 32767);
                        *index = (*index + INDEX[(nibble & 7) as usize]).clamp(0, 88);
                        samples[c].push(*predictor as i16);
                    }
                }
            }
        }
        let n = samples.iter().map(Vec::len).min().unwrap_or(0).min(per_block);
        for k in 0..n {
            for s in &samples {
                out.push(s[k]);
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ids_of_names() {
        // (the IDs the source files give these names' records)
        assert_eq!(sound_id("SM_Sandy_Game_Feet_Man_Walk"), 0xc743_f93d);
        assert_eq!(sound_id("SM_Sandy_Env_Wind"), 0x779f_652c);
        assert_eq!(sound_id("sm_sandy_env_wind"), 0x779f_652c);
        assert_eq!(sound_id(""), 0);
    }

    fn wave(format: u16, channels: u16, align: u16, bits: u16, extra: &[u8], body: &[u8]) -> Vec<u8> {
        let mut fmt = Vec::new();
        for v in [format, channels] {
            fmt.extend(v.to_le_bytes());
        }
        fmt.extend(22050u32.to_le_bytes());
        fmt.extend(44100u32.to_le_bytes());
        fmt.extend(align.to_le_bytes());
        fmt.extend(bits.to_le_bytes());
        fmt.extend(extra);
        let mut d = b"RIFF\0\0\0\0WAVEfmt ".to_vec();
        d.extend((fmt.len() as u32).to_le_bytes());
        d.extend(&fmt);
        d.extend(b"data");
        d.extend((body.len() as u32).to_le_bytes());
        d.extend(body);
        d
    }

    #[test]
    fn waves() {
        let pcm = wave(1, 1, 2, 16, &[], &[1, 0, 0xff, 0xff]);
        assert_eq!(read_sample(&pcm), Some(Sample::Pcm { rate: 22050, channels: 1, samples: vec![1, -1] }));
        // (one mono block: the first sample 100 at step index 0, then nibbles 7 (+11, the step to 16) and 0xf (−30), then zeros)
        let adpcm = wave(0x11, 1, 8, 4, &[2, 0, 9, 0], &[100, 0, 0, 0, 0xf7, 0, 0, 0]);
        let Some(Sample::Pcm { samples, .. }) = read_sample(&adpcm) else { panic!() };
        assert_eq!(samples, vec![100, 111, 81, 85, 88, 91, 94, 96, 98]);
        let mp3 = wave(0x55, 2, 1, 0, &[], &[0xff, 0xfb, 1]);
        assert_eq!(read_sample(&mp3), Some(Sample::Mp3(&[0xff, 0xfb, 1])));
        assert_eq!(read_sample(b"OggS rest"), Some(Sample::Ogg(b"OggS rest")));
    }

    #[test]
    fn a_bank() {
        let mut d = vec![0xb, 0, 0, 0, 2, 0, 0, 0];
        let mut r = vec![0u8; 0xe4];
        r[0] = 0x11;
        r[1] = 0x80 | 50;
        r[2] = 255;
        r[3] = 255;
        r[6] = 0xa3;
        r[7] = 1;
        r[12] = 150;
        r[16..20].copy_from_slice(&7u32.to_le_bytes());
        r[20..24].copy_from_slice(&9u32.to_le_bytes());
        d.extend(&r);
        let name: Vec<u8> = b"sfx/a.wav\0".iter().map(|c| c.rotate_right(3)).collect();
        d.extend((name.len() as u32).to_le_bytes());
        d.extend(&name);
        d.extend(9u32.to_le_bytes());
        let mut r = vec![0u8; 0xe4];
        r[16..20].copy_from_slice(&9u32.to_le_bytes());
        d.extend(&r);
        let bank = parse_bank(&d).unwrap();
        assert_eq!(bank.len(), 2);
        let a = &bank[0];
        assert_eq!((a.id, a.listed, a.has_file, a.chance, a.stop_others, a.priority, a.pass_delay), (7, true, true, 50, true, 1, true));
        assert_eq!((a.alert_radius, a.length, a.volume_max), (0.3, 1.5, 1.0));
        assert_eq!((a.file.as_str(), a.children.as_slice(), a.variants[0]), ("sfx/a.wav", &[9][..], 9));
        assert_eq!(bank[1].id, 9);
    }
}
