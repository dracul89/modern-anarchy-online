//! The pictures of the markup's `<img src=...>`, as `HTMLParser_c` finds them (GUI.dll FUN_1015efd3).
//!
//! The source is `rdb://` or `tdb://` (either case) and after it an expression that gives a number. `rdb://<n>` is the
//! icon `n` of the resource database (type 0xf6958: `GuiResourceManager_t::GetRDBTexture`, Interfaces.dll 0x1000b6ff,
//! names it `e_RDB_Res_Icon`), `tdb://<n>` the skin's picture number `n` (`GetGuiTexture`), and a number of 0 is the
//! skin's picture 0xb9 whichever the prefix. The skin's pictures are numbered by AFCM.dll's table of their names, and
//! `id:<NAME>` is the expression for a name's number, so `tdb://id:GFX_GUI_NPCCHAT_BULLET` is that picture.
//!
//! Here the skin is read by name, and AFCM's table is not carried: of bare `tdb://` numbers only the four the
//! original's own markup writes are known (FUN_1003f534: 0xb7, 0xb8, 0xb9, 0xe3). A picture is drawn at its own size.

use std::collections::HashMap;
use std::sync::{Arc, Mutex, OnceLock};

/// An icon's pixels by its number: width, height, RGBA.
type Icons = Box<dyn Fn(u32) -> Option<(usize, usize, Vec<u8>)> + Send + Sync>;

static ICONS: OnceLock<Icons> = OnceLock::new();

/// Where `rdb://` pictures come from (set once, when the game's database is open).
pub fn set_icons(icons: impl Fn(u32) -> Option<(usize, usize, Vec<u8>)> + Send + Sync + 'static) {
    let _ = ICONS.set(Box::new(icons));
}

/// What a source names.
#[derive(Clone, Debug, PartialEq)]
pub enum Source {
    /// An icon of the resource database.
    Icon(u32),
    /// A picture of the skin, by name.
    Sprite(String),
}

/// The skin's picture of number 0xb9, which a number of 0 gives.
const DEFAULT: &str = "GFX_GUI_DEFAULT_ICON48";

/// The numbered skin pictures the original's own markup writes.
const NUMBERED: [(u32, &str); 4] = [(0xb7, "GFX_GUI_DEFAULTBUFFICON"), (0xb8, "GFX_GUI_DEFAULT_CHARREF"), (0xb9, DEFAULT), (0xe3, "GFX_GUI_ICON_MACRO")];

/// What `src` names; None when it is no picture (neither prefix, nothing after it, or no number to be had from it), and
/// the tag then stays text.
pub fn source(src: &str) -> Option<Source> {
    let prefix = src.get(..6)?;
    let rdb = prefix.eq_ignore_ascii_case("rdb://");
    if !rdb && !prefix.eq_ignore_ascii_case("tdb://") {
        return None;
    }
    let rest = src[6..].trim();
    if let Some(name) = rest.strip_prefix("id:") {
        // (a name's number is only known here for the skin's own pictures, by the name itself)
        return (!rdb && !name.is_empty() && name.bytes().all(|c| c.is_ascii_alphanumeric() || c == b'_')).then(|| Source::Sprite(name.to_string()));
    }
    let number = match rest.strip_prefix("0x") {
        Some(hex) => u32::from_str_radix(hex, 16).ok()?,
        None => rest.parse::<i64>().ok()? as u32,
    };
    match (number, rdb) {
        (0, _) => Some(Source::Sprite(DEFAULT.to_string())),
        (n, true) => Some(Source::Icon(n)),
        (n, false) => NUMBERED.iter().find(|(k, _)| *k == n).map(|(_, name)| Source::Sprite(name.to_string())),
    }
}

type IconCache = Arc<Mutex<HashMap<u32, Option<(egui::TextureHandle, egui::Vec2)>>>>;

/// The picture of a source, uploaded on first use, and its size.
pub fn picture(ctx: &egui::Context, src: &str) -> Option<(egui::TextureId, egui::Vec2)> {
    match source(src)? {
        Source::Sprite(name) => crate::skin::skin(ctx)?.get(&name).map(|s| (s.texture.id(), s.size)),
        Source::Icon(id) => {
            let cache: IconCache = ctx.data_mut(|d| d.get_temp_mut_or_default::<IconCache>(egui::Id::new("markup-icons")).clone());
            let mut cache = cache.lock().unwrap();
            cache
                .entry(id)
                .or_insert_with(|| {
                    let (width, height, rgba) = ICONS.get()?(id)?;
                    let texture = ctx.load_texture(format!("markup-icon-{id}"), egui::ColorImage::from_rgba_unmultiplied([width, height], &rgba), egui::TextureOptions::LINEAR);
                    Some((texture, egui::vec2(width as f32, height as f32)))
                })
                .as_ref()
                .map(|(t, size)| (t.id(), *size))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_source_is_a_prefix_and_a_number() {
        assert_eq!(source("rdb://4242"), Some(Source::Icon(4242)));
        assert_eq!(source("RDB://4242"), Some(Source::Icon(4242)));
        assert_eq!(source("tdb://id:GFX_GUI_NPCCHAT_BULLET"), Some(Source::Sprite("GFX_GUI_NPCCHAT_BULLET".into())));
        assert_eq!(source("tdb://185"), Some(Source::Sprite("GFX_GUI_DEFAULT_ICON48".into())));
        // (0 is the skin's default icon, for an icon too)
        assert_eq!(source("rdb://0"), Some(Source::Sprite("GFX_GUI_DEFAULT_ICON48".into())));
        assert_eq!(source("tdb://0"), Some(Source::Sprite("GFX_GUI_DEFAULT_ICON48".into())));
    }

    #[test]
    fn what_is_no_picture_stays_text() {
        assert_eq!(source("4242"), None);
        assert_eq!(source("http://x/y.png"), None);
        assert_eq!(source("rdb://"), None);
        assert_eq!(source("rdb://icon"), None);
        assert_eq!(source("tdb://id:"), None);
    }
}
