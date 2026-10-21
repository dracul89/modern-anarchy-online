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
    Ok(Texture { width: rgba.width(), height: rgba.height(), format, rgba: rgba.into_raw() })
}
