//! The bitmap resources of a Windows program file: the original launcher (`Anarchy.exe`) keeps its buttons' pictures
//! there (`RT_BITMAP` resources, device-independent bitmaps without a file header), and the new launcher draws its
//! buttons with them when the game folder has the file.

/// A picture: width, height and RGBA bytes, top row first.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Bitmap {
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
}

fn u16_at(d: &[u8], at: usize) -> Option<u32> {
    Some(u16::from_le_bytes(d.get(at..at + 2)?.try_into().ok()?) as u32)
}

fn u32_at(d: &[u8], at: usize) -> Option<u32> {
    Some(u32::from_le_bytes(d.get(at..at + 4)?.try_into().ok()?))
}

/// The file's sections (virtual address, virtual size, file offset) and its resource directory's virtual address.
fn layout(file: &[u8]) -> Option<(Vec<(u32, u32, u32)>, u32)> {
    if file.get(..2)? != b"MZ" {
        return None;
    }
    let pe = u32_at(file, 0x3c)? as usize;
    if file.get(pe..pe + 4)? != b"PE\0\0" {
        return None;
    }
    let sections = u16_at(file, pe + 6)? as usize;
    let optional = pe + 24;
    let optional_size = u16_at(file, pe + 20)? as usize;
    // (the data directories follow 96 bytes of a 32-bit optional header, 112 of a 64-bit one; the third is the resources')
    let directories = optional + if u16_at(file, optional)? == 0x20b { 112 } else { 96 };
    let resources = u32_at(file, directories + 2 * 8)?;
    let table = optional + optional_size;
    let mut out = Vec::new();
    for i in 0..sections {
        let s = table + i * 40;
        out.push((u32_at(file, s + 12)?, u32_at(file, s + 8)?.max(u32_at(file, s + 16)?), u32_at(file, s + 20)?));
    }
    Some((out, resources))
}

/// Where a virtual address is in the file.
fn offset(sections: &[(u32, u32, u32)], address: u32) -> Option<usize> {
    sections.iter().find(|(start, size, _)| address >= *start && address < start + size).map(|(start, _, raw)| (address - start + raw) as usize)
}

/// The entries of a resource directory: each one's id (none for a named one) and what it points at (a directory when the
/// top bit is set).
fn entries(file: &[u8], base: usize, at: usize) -> Vec<(Option<u32>, u32)> {
    let (Some(named), Some(ids)) = (u16_at(file, base + at + 12), u16_at(file, base + at + 14)) else { return Vec::new() };
    (0..(named + ids) as usize)
        .filter_map(|i| {
            let e = base + at + 16 + i * 8;
            let (name, to) = (u32_at(file, e)?, u32_at(file, e + 4)?);
            Some(((name & 0x8000_0000 == 0).then_some(name), to))
        })
        .collect()
}

/// The bitmap resources of a program file by their ids (the first language of each).
pub fn bitmaps(file: &[u8]) -> Vec<(u32, Bitmap)> {
    let Some((sections, resources)) = layout(file) else { return Vec::new() };
    let Some(base) = offset(&sections, resources) else { return Vec::new() };
    let mut out = Vec::new();
    // (types, then ids, then languages: `RT_BITMAP` is type 2)
    for (kind, to) in entries(file, base, 0) {
        if kind != Some(2) || to & 0x8000_0000 == 0 {
            continue;
        }
        for (id, to) in entries(file, base, (to & 0x7fff_ffff) as usize) {
            let Some(id) = id else { continue };
            if to & 0x8000_0000 == 0 {
                continue;
            }
            let Some(&(_, data)) = entries(file, base, (to & 0x7fff_ffff) as usize).first() else { continue };
            let entry = base + data as usize;
            let (Some(address), Some(size)) = (u32_at(file, entry), u32_at(file, entry + 4)) else { continue };
            let Some(start) = offset(&sections, address) else { continue };
            if let Some(bitmap) = file.get(start..start + size as usize).and_then(dib) {
                out.push((id, bitmap));
            }
        }
    }
    out
}

/// A device-independent bitmap (a `BITMAPINFOHEADER`, a palette for 8 bits and under, the rows bottom first unless the
/// height is negative, each padded to four bytes): 8, 24 and 32 bits a pixel, uncompressed.
pub fn dib(d: &[u8]) -> Option<Bitmap> {
    let header = u32_at(d, 0)? as usize;
    let width = u32_at(d, 4)? as i32;
    let height = u32_at(d, 8)? as i32;
    let bits = u16_at(d, 14)?;
    let compression = u32_at(d, 16)?;
    if header < 40 || width <= 0 || height == 0 || compression != 0 || !matches!(bits, 8 | 24 | 32) {
        return None;
    }
    let colours = if bits == 8 { Some(u32_at(d, 32)?).filter(|&n| n != 0).unwrap_or(256) as usize } else { 0 };
    let palette = d.get(header..header + colours * 4)?;
    let pixels = header + colours * 4;
    let (w, h) = (width as usize, height.unsigned_abs() as usize);
    let stride = (w * bits as usize / 8).div_ceil(4) * 4;
    let mut rgba = Vec::with_capacity(w * h * 4);
    for row in 0..h {
        let source = if height > 0 { h - 1 - row } else { row };
        let line = d.get(pixels + source * stride..pixels + source * stride + stride)?;
        for x in 0..w {
            let (b, g, r) = match bits {
                8 => {
                    let c = palette.get(line[x] as usize * 4..line[x] as usize * 4 + 3)?;
                    (c[0], c[1], c[2])
                }
                24 => (line[x * 3], line[x * 3 + 1], line[x * 3 + 2]),
                _ => (line[x * 4], line[x * 4 + 1], line[x * 4 + 2]),
            };
            rgba.extend_from_slice(&[r, g, b, 255]);
        }
    }
    Some(Bitmap { width: w as u32, height: h as u32, rgba })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A two by two 24-bit bitmap: rows bottom first, blue green red, each row padded to four bytes.
    #[test]
    fn a_dib_reads_bottom_row_first() {
        let mut d = Vec::new();
        d.extend_from_slice(&40u32.to_le_bytes());
        d.extend_from_slice(&2i32.to_le_bytes());
        d.extend_from_slice(&2i32.to_le_bytes());
        d.extend_from_slice(&1u16.to_le_bytes());
        d.extend_from_slice(&24u16.to_le_bytes());
        d.extend_from_slice(&[0; 24]);
        // (the bottom row: blue, white; the top row: red, green)
        d.extend_from_slice(&[255, 0, 0, 255, 255, 255, 0, 0]);
        d.extend_from_slice(&[0, 0, 255, 0, 255, 0, 0, 0]);
        let b = dib(&d).unwrap();
        assert_eq!((b.width, b.height), (2, 2));
        assert_eq!(b.rgba, vec![255, 0, 0, 255, 0, 255, 0, 255, 0, 0, 255, 255, 255, 255, 255, 255]);
        assert_eq!(dib(&d[..50]), None);
    }

    /// A program file with one section holding a resource directory with one bitmap, id 176.
    #[test]
    fn a_program_files_bitmaps_are_found_by_id() {
        let mut dib_bytes = Vec::new();
        dib_bytes.extend_from_slice(&40u32.to_le_bytes());
        dib_bytes.extend_from_slice(&1i32.to_le_bytes());
        dib_bytes.extend_from_slice(&1i32.to_le_bytes());
        dib_bytes.extend_from_slice(&1u16.to_le_bytes());
        dib_bytes.extend_from_slice(&24u16.to_le_bytes());
        dib_bytes.extend_from_slice(&[0; 24]);
        dib_bytes.extend_from_slice(&[10, 20, 30, 0]);
        // (the resource section, at virtual address 0x2000 and file offset 0x400)
        let directory = |id: u32, to: u32| {
            let mut d = vec![0u8; 12];
            d.extend_from_slice(&0u16.to_le_bytes());
            d.extend_from_slice(&1u16.to_le_bytes());
            d.extend_from_slice(&id.to_le_bytes());
            d.extend_from_slice(&to.to_le_bytes());
            d
        };
        let mut section = Vec::new();
        section.extend(directory(2, 0x8000_0018));
        section.extend(directory(176, 0x8000_0030));
        section.extend(directory(1033, 0x48));
        section.extend_from_slice(&0x2058u32.to_le_bytes());
        section.extend_from_slice(&(dib_bytes.len() as u32).to_le_bytes());
        section.extend_from_slice(&[0; 8]);
        section.extend_from_slice(&dib_bytes);
        let mut file = vec![0u8; 0x400];
        file[..2].copy_from_slice(b"MZ");
        file[0x3c..0x40].copy_from_slice(&0x80u32.to_le_bytes());
        file[0x80..0x84].copy_from_slice(b"PE\0\0");
        file[0x86..0x88].copy_from_slice(&1u16.to_le_bytes());
        file[0x94..0x96].copy_from_slice(&224u16.to_le_bytes());
        file[0x98..0x9a].copy_from_slice(&0x10bu16.to_le_bytes());
        // (the third data directory: 0x98 + 96 + 16)
        file[0x108..0x10c].copy_from_slice(&0x2000u32.to_le_bytes());
        let table = 0x98 + 224;
        file[table + 8..table + 12].copy_from_slice(&(section.len() as u32).to_le_bytes());
        file[table + 12..table + 16].copy_from_slice(&0x2000u32.to_le_bytes());
        file[table + 16..table + 20].copy_from_slice(&(section.len() as u32).to_le_bytes());
        file[table + 20..table + 24].copy_from_slice(&0x400u32.to_le_bytes());
        file.extend_from_slice(&section);
        let found = bitmaps(&file);
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].0, 176);
        assert_eq!(found[0].1, Bitmap { width: 1, height: 1, rgba: vec![30, 20, 10, 255] });
        assert!(bitmaps(b"not a program").is_empty());
    }
}
