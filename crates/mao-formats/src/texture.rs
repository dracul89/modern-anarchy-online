//! Textures: the resource database's `RDBTexture_t` objects (types 0xf6954,
//! 0xf6959, 0xf695b, 0xf6960, 0xf6961, 0xf6963, 0xf6964, 0xf6967, 0xf6968).
//!
//! Derived from `RDBTexture_t::ReadBlob` (`AnarchyReimagined`:
//! `dropin/displaysystem/src/rdb.rs`). An object is the usual 12-byte head
//! (type, instance, revision) and then an ordinary image file — BMP, PNG or
//! JPEG. Many are stored with the file's first four bytes replaced by a
//! small number: below 16, the signature was stripped to keep the files from
//! looking like images, and an even number means PNG, an odd one JPEG. The
//! other bytes of the signature are intact.

use std::fmt;

/// The head of every database object: type, instance, revision.
pub const OBJECT_HEAD: usize = 12;

#[derive(Debug)]
pub enum Error {
    /// The object is too short to hold an image.
    Short,
    /// The image data wasn't a readable BMP, PNG or JPEG.
    Image(image::ImageError),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Short => write!(f, "the texture object is too short"),
            Error::Image(e) => write!(f, "{e}"),
        }
    }
}

impl std::error::Error for Error {}

impl From<image::ImageError> for Error {
    fn from(e: image::ImageError) -> Error {
        Error::Image(e)
    }
}

/// The file format an object's image is in.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Format {
    Bmp,
    Png,
    Jpeg,
}

/// A decoded texture: 8-bit RGBA, row by row from the top.
#[derive(Clone, Debug)]
pub struct Texture {
    pub width: u32,
    pub height: u32,
    pub format: Format,
    pub rgba: Vec<u8>,
    /// How many mip levels `rgba` holds, one after the other, the full image first (1: just the image).
    pub mips: u32,
}

impl Texture {
    /// The texture with its mip chain appended to `rgba` (each level a box-filtered half of the one before, down to 1x1).
    /// Done where the texture is decoded, which can be many threads at once, so that the upload need not.
    pub fn with_mips(self) -> Texture {
        if self.mips > 1 {
            return self;
        }
        let (rgba, mips) = mip_chain(&self.rgba, self.width, self.height);
        Texture { rgba, mips, ..self }
    }
}

/// The image followed by its mip levels, and how many levels that is.
pub fn mip_chain(image: &[u8], width: u32, height: u32) -> (Vec<u8>, u32) {
    let mut data = image.to_vec();
    let (mut w, mut h) = (width as usize, height as usize);
    let mut level_start = 0;
    let mut levels = 1;
    while w > 1 || h > 1 {
        let (nw, nh) = ((w / 2).max(1), (h / 2).max(1));
        let mut next = Vec::with_capacity(nw * nh * 4);
        for y in 0..nh {
            for x in 0..nw {
                let mut sum = [0u32; 4];
                for (dx, dy) in [(0, 0), (1, 0), (0, 1), (1, 1)] {
                    let (sx, sy) = ((x * 2 + dx).min(w - 1), (y * 2 + dy).min(h - 1));
                    let at = level_start + (sy * w + sx) * 4;
                    for c in 0..4 {
                        sum[c] += data[at + c] as u32;
                    }
                }
                next.extend(sum.map(|v| ((v + 2) / 4) as u8));
            }
        }
        level_start = data.len();
        data.extend_from_slice(&next);
        (w, h) = (nw, nh);
        levels += 1;
    }
    (data, levels)
}

/// The image file in a texture object's bytes, with its signature restored,
/// and its format.
pub fn image_file(object: &[u8]) -> Result<(Vec<u8>, Format), Error> {
    image_file_after(object, OBJECT_HEAD)
}

/// As [`image_file`], the image starting `head` bytes in.
pub fn image_file_after(object: &[u8], head: usize) -> Result<(Vec<u8>, Format), Error> {
    let mut file = object.get(head..).filter(|f| f.len() >= 4).ok_or(Error::Short)?.to_vec();
    let first = u32::from_le_bytes(file[..4].try_into().unwrap());
    if first & 0xffff_fff0 == 0 {
        // A stripped signature: even is PNG ("\x89PNG"), odd JPEG (FF D8 FF E0).
        let signature: [u8; 4] = if first & 1 == 0 { [0x89, b'P', b'N', b'G'] } else { [0xff, 0xd8, 0xff, 0xe0] };
        file[..4].copy_from_slice(&signature);
    }
    // The original decides by the first bytes: 'B' is a BMP, else a second
    // byte of 'P' a PNG and anything else a JPEG.
    let format = if file[0] == b'B' {
        Format::Bmp
    } else if file[1] == b'P' {
        Format::Png
    } else {
        Format::Jpeg
    };
    Ok((file, format))
}

/// Decodes a texture object.
pub fn decode(object: &[u8]) -> Result<Texture, Error> {
    decode_after(object, OBJECT_HEAD)
}

/// Decodes an image that starts `head` bytes into the object.
pub fn decode_after(object: &[u8], head: usize) -> Result<Texture, Error> {
    let (file, format) = image_file_after(object, head)?;
    let decoded = match format {
        Format::Bmp => image::load_from_memory_with_format(&file, image::ImageFormat::Bmp)?,
        Format::Png => image::load_from_memory_with_format(&file, image::ImageFormat::Png)?,
        Format::Jpeg => image::load_from_memory_with_format(&file, image::ImageFormat::Jpeg)?,
    };
    let rgba = decoded.to_rgba8();
    Ok(Texture { width: rgba.width(), height: rgba.height(), format, rgba: rgba.into_raw(), mips: 1 })
}

/// The header before the image in a ground texture object
/// (`RDBGroundTexture_t`, types 0xf6956, 0xf6965 and 0xf6966) after the
/// object head.
pub const GROUND_HEADER: usize = 0x18;

/// Decodes a ground texture object: the head, a 0x18-byte header, then an
/// image whose first byte, when below 16, was replaced (the rest of the
/// first four bytes made a PNG or, for an odd byte, JPEG signature). A file
/// starting with 'B' is a BMP, one with 'P' second a PNG, else a JPEG.
pub fn decode_ground(object: &[u8]) -> Result<Texture, Error> {
    let start = OBJECT_HEAD + GROUND_HEADER;
    let mut file = object.get(start..).filter(|f| f.len() >= 4).ok_or(Error::Short)?.to_vec();
    if file[0] & 0xf0 == 0 {
        let signature: [u8; 4] = if file[0] & 1 != 0 { [0xff, 0xd8, 0xff, 0xe0] } else { [0x89, b'P', b'N', b'G'] };
        file[..4].copy_from_slice(&signature);
    }
    let format = if file[0] == b'B' {
        Format::Bmp
    } else if file[1] == b'P' {
        Format::Png
    } else {
        Format::Jpeg
    };
    let decoded = image::load_from_memory_with_format(
        &file,
        match format {
            Format::Bmp => image::ImageFormat::Bmp,
            Format::Png => image::ImageFormat::Png,
            Format::Jpeg => image::ImageFormat::Jpeg,
        },
    )?;
    let rgba = decoded.to_rgba8();
    Ok(Texture { width: rgba.width(), height: rgba.height(), format, rgba: rgba.into_raw(), mips: 1 })
}
