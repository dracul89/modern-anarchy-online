//! A character's bare skin, as the original makes it: no colour is worked out anywhere. The body's cloth textures are
//! painted pure green where skin shows through, and a texture of the breed's, sex's and race's bare skin stands behind them.
//!
//! DisplaySystem's `VisualCATMesh_t` keeps up to three textures for each of the five body parts (hands, body, feet, arms,
//! legs): the mesh's own, what is worn, and the naked one `SetSkinData` asks for (FUN_10070439; its name is made by
//! FUN_1006ff7f, [`naked_name`]). FUN_1007457f takes what is worn, else the mesh's own, and FUN_10074393 lays it over
//! the naked one: every pixel that is exactly green (`0x07e0` in five, six and five bits, [`is_green`]) shows the naked
//! texture's pixel at that place, and a texture with no such pixel is used as it is. What comes out has no alpha.
//!
//! The renderer knows a texture by a type and a number, so a pair of the two is given a number of its own under the made-up
//! type [`KIND`] ([`composed`]), and `Textures` makes the picture when it is first asked for.
//!
//! Not as the original: the picture keeps its eight bits a channel (the original's is in 16 bits), and the naked texture is
//! stretched to the other's size by a plain linear filter (the original by D3DX's own).

use std::collections::HashMap;
use std::sync::Mutex;

use mao_formats::texture::Texture;

/// The type the composed pictures go by.
pub const KIND: u32 = 0xfe5c_0001;

/// The body parts that have a naked texture (`FUN_10070439` asks for the first five cloth parts: not the head).
pub const PARTS: [&str; 5] = ["hands", "body", "feet", "arms", "legs"];

#[derive(Default)]
struct Pairs {
    numbers: HashMap<((u32, u32), u32), u32>,
    pairs: Vec<((u32, u32), u32)>,
}

static PAIRS: Mutex<Option<Pairs>> = Mutex::new(None);

/// What a part wears over a naked texture, as the texture the renderer is to ask for.
pub fn composed(top: (u32, u32), naked: u32) -> (u32, u32) {
    let mut guard = PAIRS.lock().unwrap_or_else(|e| e.into_inner());
    let pairs = guard.get_or_insert_with(Pairs::default);
    let next = pairs.pairs.len() as u32;
    let number = *pairs.numbers.entry((top, naked)).or_insert(next);
    if number == next {
        pairs.pairs.push((top, naked));
    }
    (KIND, number)
}

/// The two textures of a number [`composed`] gave.
pub fn pair(number: u32) -> Option<((u32, u32), u32)> {
    PAIRS.lock().unwrap_or_else(|e| e.into_inner()).as_ref()?.pairs.get(number as usize).copied()
}

/// Whether a pixel is the original's green: `0x07e0` once its channels are cut to five, six and five bits.
pub fn is_green(p: &[u8]) -> bool {
    p[0] >> 3 == 0 && p[1] >> 2 == 63 && p[2] >> 3 == 0
}

/// The naked texture's file name (FUN_1006ff7f): the part, the breed, the sex (an Atrox and a sexless body are male) and, for a
/// Solitus, the race. Type 0xf695b.
pub fn naked_name(part: &str, breed: u8, sex: u8, race: u8) -> Option<String> {
    let breed_name = *["solitus", "opifex", "nanomage", "athrox"].get(usize::from(breed).checked_sub(1)?)?;
    let sex_name = match sex {
        _ if breed == 4 => "male",
        1 | 2 => "male",
        3 => "female",
        _ => return None,
    };
    let race_name = match (breed, race) {
        (1, 2) => "_african",
        (1, 3) => "_asian",
        (1, _) => "_caucation",
        _ => "",
    };
    Some(format!("{part}_{breed_name}{sex_name}{race_name}_naked.png"))
}

/// The race a head mesh's file name says (GUI.dll's and Gamecode's lists of heads, FUN_100c3239: each set of heads has one).
pub fn race_of_head(name: &str) -> u8 {
    if name.contains("_african") {
        2
    } else if name.contains("_asian") {
        3
    } else {
        1
    }
}

/// The breed and sex a head mesh's file name is of (`head_solitusfemale_african03.abiff`: 1, 3). For a body the client is told
/// neither of (remains, a morph); the original reads them from the character's stats 4 and 59.
pub fn body_of_head(name: &str) -> Option<(u8, u8)> {
    let name = name.to_ascii_lowercase();
    let rest = name.strip_prefix("head_")?;
    if rest.starts_with("athrox") {
        return Some((4, 1));
    }
    let (breed, rest) = [("solitus", 1), ("opifex", 2), ("nano", 3)].iter().find_map(|&(word, breed)| Some((breed, rest.strip_prefix(word)?)))?;
    if rest.starts_with("female") {
        Some((breed, 3))
    } else if rest.starts_with("male") {
        Some((breed, 2))
    } else {
        None
    }
}

/// A texture's pixel at a place from 0 to 1 across each way, between its four nearest.
fn sampled(t: &Texture, u: f32, v: f32) -> [u8; 3] {
    let (w, h) = (t.width as usize, t.height as usize);
    let (x, y) = ((u * w as f32 - 0.5).max(0.0), (v * h as f32 - 0.5).max(0.0));
    let (x0, y0) = ((x as usize).min(w - 1), (y as usize).min(h - 1));
    let (x1, y1) = ((x0 + 1).min(w - 1), (y0 + 1).min(h - 1));
    let (fx, fy) = (x.fract(), y.fract());
    let at = |x: usize, y: usize, c: usize| t.rgba[(y * w + x) * 4 + c] as f32;
    std::array::from_fn(|c| {
        let top = at(x0, y0, c) * (1.0 - fx) + at(x1, y0, c) * fx;
        let bottom = at(x0, y1, c) * (1.0 - fx) + at(x1, y1, c) * fx;
        (top * (1.0 - fy) + bottom * fy + 0.5) as u8
    })
}

/// FUN_10074393: `top` over `naked`, the green showing the naked texture; none when `top` has no green, which is then used as
/// it is.
pub fn compose(top: &Texture, naked: &Texture) -> Option<Texture> {
    let (w, h) = (top.width as usize, top.height as usize);
    let image = top.rgba.get(..w * h * 4)?;
    if naked.width == 0 || naked.height == 0 || naked.rgba.len() < naked.width as usize * naked.height as usize * 4 || !image.chunks_exact(4).any(is_green) {
        return None;
    }
    let mut rgba = Vec::with_capacity(w * h * 4);
    for (k, p) in image.chunks_exact(4).enumerate() {
        let c = if is_green(p) { sampled(naked, ((k % w) as f32 + 0.5) / w as f32, ((k / w) as f32 + 0.5) / h as f32) } else { [p[0], p[1], p[2]] };
        rgba.extend_from_slice(&[c[0], c[1], c[2], 255]);
    }
    Some(Texture { width: top.width, height: top.height, format: top.format, rgba, mips: 1 })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn picture(width: u32, height: u32, pixels: &[[u8; 4]]) -> Texture {
        Texture { width, height, format: mao_formats::texture::Format::Png, rgba: pixels.concat(), mips: 1 }
    }

    /// Only the exact green is the skin's: a green a little off stays what it is, as the 16-bit test leaves it.
    #[test]
    fn a_head_names_its_body() {
        assert_eq!(body_of_head("head_solitusfemale_african03.abiff"), Some((1, 3)));
        assert_eq!(body_of_head("head_solitusmale16.abiff"), Some((1, 2)));
        assert_eq!(body_of_head("head_opifexfemale37.abiff"), Some((2, 3)));
        assert_eq!(body_of_head("head_nanomale02.abiff"), Some((3, 2)));
        assert_eq!(body_of_head("head_athrox05.abiff"), Some((4, 1)));
        assert_eq!(body_of_head("helmet_solitusmale.abiff"), None);
    }

    #[test]
    fn the_green_is_exact() {
        assert!(is_green(&[0, 255, 0]) && is_green(&[7, 252, 7]));
        assert!(!is_green(&[8, 255, 0]) && !is_green(&[0, 251, 0]) && !is_green(&[0, 255, 8]) && !is_green(&[60, 200, 60]));
    }

    #[test]
    fn the_naked_texture_shows_where_the_green_is() {
        let top = picture(2, 1, &[[0, 255, 0, 40], [200, 100, 50, 40]]);
        let naked = picture(1, 1, &[[210, 160, 120, 255]]);
        let made = compose(&top, &naked).expect("it has green");
        // (the skin in the green, the cloth elsewhere, and no alpha left)
        assert_eq!(made.rgba, [210, 160, 120, 255, 200, 100, 50, 255]);
        // (nothing green: used as it is)
        assert!(compose(&picture(1, 1, &[[1, 2, 3, 4]]), &naked).is_none());
    }

    /// The names as the game's files have them.
    #[test]
    fn the_naked_textures_names() {
        assert_eq!(naked_name("hands", 1, 2, 2).as_deref(), Some("hands_solitusmale_african_naked.png"));
        assert_eq!(naked_name("body", 1, 3, 1).as_deref(), Some("body_solitusfemale_caucation_naked.png"));
        assert_eq!(naked_name("legs", 3, 3, 1).as_deref(), Some("legs_nanomagefemale_naked.png"));
        assert_eq!(naked_name("feet", 4, 1, 3).as_deref(), Some("feet_athroxmale_naked.png"));
        assert_eq!((naked_name("arms", 0, 2, 1), naked_name("arms", 5, 2, 1)), (None, None));
        assert_eq!((race_of_head("head_solitusmale_african03.abiff"), race_of_head("head_solitusfemale_asian01.abiff"), race_of_head("head_opifexmale12.abiff")), (2, 3, 1));
    }

    /// One number for one pair, and the pair back from it.
    #[test]
    fn a_pair_has_one_number() {
        let (kind, number) = composed((0xf6954, 77), 88);
        assert_eq!((kind, composed((0xf6954, 77), 88).1), (KIND, number));
        assert_ne!(composed((0xf6954, 78), 88).1, number);
        assert_eq!(pair(number), Some(((0xf6954, 77), 88)));
    }
}
