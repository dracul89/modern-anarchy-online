//! A Photoshop file's flattened picture: the last section of a `.psd`, the whole document as it looked when it was
//! saved. Planet maps' makers keep their pictures so (the Atlas of Rubi-Ka's source is four of them), and
//! `planetmap::pack` makes a map's level of one. Only what those need is read: version 1, RGB, 8 bits a channel, the
//! rows raw or run-length packed (PackBits). The channels after the third (saved selections) are passed over.
//!
//! The file: `8BPS`, the version, six unused bytes, the channels, the height, the width, the depth and the colour
//! mode (26 bytes, big-endian); three sections each behind its length (the colour table, the resources, the layers);
//! then the compression (0 raw, 1 packed) and the picture, channel after channel. Packed, every row of every channel
//! has its byte count first, all the counts before the first row.

fn u16be(data: &[u8], at: usize) -> Option<usize> {
    Some(u16::from_be_bytes(data.get(at..at + 2)?.try_into().ok()?) as usize)
}

fn u32be(data: &[u8], at: usize) -> Option<usize> {
    Some(u32::from_be_bytes(data.get(at..at + 4)?.try_into().ok()?) as usize)
}

/// A PackBits row put after `out`: a count byte below 128 is that many bytes and one more as they are, above 128 the
/// next byte 257 less the count times, 128 nothing.
fn unpack(row: &[u8], out: &mut Vec<u8>) {
    let mut i = 0;
    while i < row.len() {
        let n = row[i] as usize;
        i += 1;
        if n < 128 {
            let end = (i + n + 1).min(row.len());
            out.extend_from_slice(&row[i..end]);
            i = end;
        } else if n > 128 {
            if let Some(&b) = row.get(i) {
                out.extend(std::iter::repeat_n(b, 257 - n));
            }
            i += 1;
        }
    }
}

/// The flattened picture (opaque); none for a file that is not an 8-bit RGB `.psd` or is cut short.
pub fn flattened(data: &[u8]) -> Option<image::RgbaImage> {
    if data.get(..4)? != b"8BPS" || u16be(data, 4)? != 1 {
        return None;
    }
    let (channels, height, width) = (u16be(data, 12)?, u32be(data, 14)?, u32be(data, 18)?);
    if u16be(data, 22)? != 8 || u16be(data, 24)? != 3 || channels < 3 || width == 0 || height == 0 {
        return None;
    }
    let mut at = 26usize;
    for _ in 0..3 {
        at = at.checked_add(4 + u32be(data, at)?)?;
    }
    let compression = u16be(data, at)?;
    at += 2;
    let mut planes: Vec<Vec<u8>> = Vec::new();
    match compression {
        0 => {
            for c in 0..3 {
                planes.push(data.get(at + c * width * height..at + (c + 1) * width * height)?.to_vec());
            }
        }
        1 => {
            let counts = at;
            at += channels * height * 2;
            for c in 0..3 {
                let mut plane = Vec::with_capacity(width * height);
                for y in 0..height {
                    let n = u16be(data, counts + (c * height + y) * 2)?;
                    let start = plane.len();
                    unpack(data.get(at..at + n)?, &mut plane);
                    if plane.len() != start + width {
                        return None;
                    }
                    at += n;
                }
                planes.push(plane);
            }
        }
        _ => return None,
    }
    let mut out = image::RgbaImage::new(width as u32, height as u32);
    for (i, p) in out.pixels_mut().enumerate() {
        *p = image::Rgba([planes[0][i], planes[1][i], planes[2][i], 255]);
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn header(channels: u16, width: u32, height: u32, compression: u16) -> Vec<u8> {
        let mut f = b"8BPS".to_vec();
        f.extend(1u16.to_be_bytes());
        f.extend([0; 6]);
        f.extend(channels.to_be_bytes());
        f.extend(height.to_be_bytes());
        f.extend(width.to_be_bytes());
        f.extend(8u16.to_be_bytes());
        f.extend(3u16.to_be_bytes());
        // (no colour table, four bytes of resources, no layers)
        f.extend(0u32.to_be_bytes());
        f.extend(4u32.to_be_bytes());
        f.extend([9; 4]);
        f.extend(0u32.to_be_bytes());
        f.extend(compression.to_be_bytes());
        f
    }

    /// A raw picture and a packed one with a fourth channel: the pixels come out, the fourth channel does not count.
    #[test]
    fn the_flattened_picture_is_read() {
        // 3 by 2, raw: red rows 1 2 3 / 4 5 6, green ten more, blue twenty more
        let mut raw = header(3, 3, 2, 0);
        for add in [0u8, 10, 20] {
            raw.extend((1..=6).map(|v| v + add));
        }
        let picture = flattened(&raw).unwrap();
        assert_eq!(picture.dimensions(), (3, 2));
        assert_eq!(picture.get_pixel(0, 0).0, [1, 11, 21, 255]);
        assert_eq!(picture.get_pixel(2, 1).0, [6, 16, 26, 255]);

        // 4 by 2, packed, four channels: a row as a run (0xfd: four times), a row as it is (0x03: four bytes), and a
        // row as a run of two and two as they are
        let rows: [&[u8]; 8] = [&[0xfd, 7], &[0x03, 1, 2, 3, 4], &[0xff, 5, 0x01, 6, 7], &[0xfd, 8], &[0xfd, 9], &[0xfd, 10], &[0xfd, 99], &[0xfd, 98]];
        let mut packed = header(4, 4, 2, 1);
        for r in rows {
            packed.extend((r.len() as u16).to_be_bytes());
        }
        for r in rows {
            packed.extend(r);
        }
        let picture = flattened(&packed).unwrap();
        assert_eq!(picture.dimensions(), (4, 2));
        assert_eq!(picture.get_pixel(0, 0).0, [7, 5, 9, 255]);
        assert_eq!(picture.get_pixel(3, 0).0, [7, 7, 9, 255]);
        assert_eq!(picture.get_pixel(2, 1).0, [3, 8, 10, 255]);

        // (a row of the wrong length, another kind of file, a file cut short: none)
        let mut bad = packed.clone();
        // (the first row's run made three long: after the header and the eight counts)
        bad[header(4, 4, 2, 1).len() + 16] = 0xfe;
        assert!(flattened(&bad).is_none());
        assert!(flattened(b"8BPX").is_none());
        assert!(flattened(&raw[..raw.len() - 1]).is_none());
        let mut deep = raw.clone();
        deep[23] = 16;
        assert!(flattened(&deep).is_none());
    }
}
