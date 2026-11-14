//! The interface's colours, read from the same files the original client
//! (and the AOU mod's themes) use: a folder with `GUIColors.xml` and
//! `TextColors.xml`.
//!
//! The original's `GUIColors.xml` names six colours: `Default` (the tint of a
//! control at rest), `Selected`, `Hover` and the three text colours. The AOU
//! themes keep the same names with other values (the Translucent theme is
//! dark navy with neon cyan). A theme is chosen in the settings (`theme =`):
//! `built-in`, `original` (the game's own `cd_image/gui/Default`), one of the
//! AOU mod's (`aou_translucent`, `aou_cyt`, `aou_slim_cni`, `aou_slim_cnt`,
//! found under `aou_mod =`), or the path of any such folder.

use std::path::{Path, PathBuf};
use std::sync::OnceLock;

/// An RGB colour.
pub type Rgb = [u8; 3];

/// The colours the interface is drawn with.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Palette {
    pub name: String,
    /// The fill of windows and panels (dark, so text reads on the world).
    pub panel: Rgb,
    /// Borders and the selected state.
    pub accent: Rgb,
    /// What the pointer is over.
    pub hover: Rgb,
    pub text: Rgb,
    pub text_selected: Rgb,
    /// `GUIColors.xml`'s `Default` and `Selected` as they are (`ControlCenterModule_c::LoadGUIColors`, GUI.dll 0x1006c13d, makes
    /// them colour codes 0x1000000 and 0x2000000: a button at rest and pressed).
    pub default: Rgb,
    pub selected: Rgb,
    /// The named colours of the folder's `TextColors.xml` (chat kinds `ct_*`/`ctch_*`, `CC*` text classes); empty for the built-in look.
    pub text_colors: Vec<(String, Rgb)>,
}

/// The themes the AOU mod ships, by the names the settings use.
pub const AOU_THEMES: &[(&str, &str)] = &[("aou_translucent", "AOU_Translucent"), ("aou_cyt", "AOU_CYT"), ("aou_slim_cni", "AOU_Slim_CNI"), ("aou_slim_cnt", "AOU_Slim_CNT")];

/// The game's folder, so the `original` theme can find its own GUI files.
static GAME: OnceLock<PathBuf> = OnceLock::new();

pub fn set_game_dir(game: &str) {
    let _ = GAME.set(PathBuf::from(game));
}

/// `0x0030d2ff` (only the low 24 bits count) or `#30d2ff`.
pub fn parse_color(text: &str) -> Option<Rgb> {
    let t = text.trim();
    let digits = t.strip_prefix("0x").or_else(|| t.strip_prefix("0X")).or_else(|| t.strip_prefix('#')).unwrap_or(t);
    let v = u32::from_str_radix(digits, 16).ok()?;
    Some([(v >> 16) as u8, (v >> 8) as u8, v as u8])
}

/// The value of an attribute in an XML element's text: `name="x"`.
fn attribute<'a>(element: &'a str, name: &str) -> Option<&'a str> {
    let start = element.find(&format!("{name}=\""))? + name.len() + 2;
    let end = element[start..].find('"')? + start;
    Some(&element[start..end])
}

/// The named colours of a `GUIColors.xml`.
pub fn gui_colors(xml: &str) -> Vec<(String, Rgb)> {
    xml.split('<').filter(|e| e.starts_with("Color ")).filter_map(|e| Some((attribute(e, "name")?.to_string(), parse_color(attribute(e, "color")?)?))).collect()
}

fn luminance(c: Rgb) -> f32 {
    (0.2126 * c[0] as f32 + 0.7152 * c[1] as f32 + 0.0722 * c[2] as f32) / 255.0
}

fn scale(c: Rgb, k: f32) -> Rgb {
    c.map(|v| (v as f32 * k).round().clamp(0.0, 255.0) as u8)
}

impl Palette {
    /// The look used when no theme is chosen: dark warm panels with silver and gold.
    pub fn built_in() -> Palette {
        Palette { name: "built-in".into(), panel: [18, 14, 10], accent: [212, 175, 55], hover: [240, 208, 96], text: [230, 226, 214], text_selected: [255, 255, 255], default: [212, 175, 55], selected: [240, 208, 96], text_colors: Vec::new() }
    }

    /// A palette from a `GUIColors.xml`. A theme whose `Default` is dark (the
    /// AOU Translucent) fills panels with it and draws borders in its
    /// `Selected`; one whose `Default` is light (the original's) darkens it for
    /// the panels and borders in it.
    pub fn from_gui_colors(name: &str, xml: &str) -> Option<Palette> {
        let colors = gui_colors(xml);
        let get = |n: &str| colors.iter().find(|(k, _)| k == n).map(|&(_, c)| c);
        let default = get("Default")?;
        let selected = get("Selected").unwrap_or(default);
        let dark = luminance(default) < 0.25;
        Some(Palette {
            name: name.to_string(),
            panel: if dark { default } else { scale(default, 0.14) },
            accent: if dark { selected } else { default },
            hover: get("Hover").unwrap_or(selected),
            text: get("Text").unwrap_or(selected),
            text_selected: get("TextSelected").unwrap_or([255, 255, 255]),
            default,
            selected,
            text_colors: Vec::new(),
        })
    }

    pub fn from_dir(name: &str, dir: &Path) -> Option<Palette> {
        let mut palette = Palette::from_gui_colors(name, &mao_formats::files::read_to_string(dir.join("GUIColors.xml")).ok()?)?;
        // (a skin without its own text colours takes the game's)
        let text = mao_formats::files::read_to_string(dir.join("TextColors.xml")).ok().or_else(|| mao_formats::files::read_to_string(GAME.get()?.join("cd_image/gui/Default/TextColors.xml")).ok());
        palette.text_colors = text.map(|xml| text_colors(&xml)).unwrap_or_default();
        Some(palette)
    }

    /// A named text colour (`ct_itell`, `CCLinkColor`, ...), when the theme has it.
    pub fn text(&self, name: &str) -> Option<Rgb> {
        self.text_colors.iter().find(|(k, _)| k == name).map(|&(_, c)| c)
    }
}

/// The named colours of a `TextColors.xml`: `<HTMLColor color="0x00ffff" name="ct_itell"/>`; a colour may name an earlier
/// entry (`color="yellow"`).
pub fn text_colors(xml: &str) -> Vec<(String, Rgb)> {
    let mut out: Vec<(String, Rgb)> = Vec::new();
    for element in xml.split('<').filter(|e| e.starts_with("HTMLColor ")) {
        let (Some(name), Some(value)) = (attribute(element, "name"), attribute(element, "color")) else { continue };
        let color = parse_color(value).or_else(|| out.iter().find(|(k, _)| k == value.trim()).map(|&(_, c)| c));
        if let Some(color) = color {
            out.push((name.to_string(), color));
        }
    }
    out
}

/// The skins bundled with the client (`assets/gui`: the AOU skins, each a `Gui/<skin>` folder of the original's): next to
/// the program (in a macOS application, whose program is `Contents/MacOS/mao-client`, in its `Contents/Resources`) or in
/// the folder it runs from, else the source tree's.
pub fn bundled_gui_dir() -> Option<PathBuf> {
    let beside = |below: &str| std::env::current_exe().ok().and_then(|e| e.parent().map(|p| p.join(below)));
    let candidates = [beside("assets/gui"), beside("../Resources/assets/gui"), Some(PathBuf::from("assets/gui")), Some(Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets/gui"))];
    candidates.into_iter().flatten().find(|d| d.is_dir())
}

/// The folder a `theme =` setting names, if it can be found.
pub fn theme_dir(theme: &str, aou_mod: &str) -> Option<PathBuf> {
    let theme = theme.trim();
    match theme.to_ascii_lowercase().as_str() {
        "" | "built-in" | "builtin" => None,
        "original" => GAME.get().map(|g| g.join("cd_image/gui/Default")),
        other => match AOU_THEMES.iter().find(|(name, _)| *name == other) {
            Some((_, folder)) => {
                // (the skins bundled with the client first, then the AOU mod's own folder)
                if let Some(dir) = bundled_gui_dir().map(|d| d.join(folder)).filter(|d| mao_formats::files::exists(d.join("GUIColors.xml"))) {
                    return Some(dir);
                }
                let root = if aou_mod.trim().is_empty() { std::env::var("MAO_AOU_MOD").unwrap_or_default() } else { aou_mod.to_string() };
                (!root.trim().is_empty()).then(|| mao_formats::files::find(&Path::new(root.trim()).join("Gui").join(folder)))
            }
            None => Some(PathBuf::from(theme)),
        },
    }
}

/// The palette for a `theme =` setting; the built-in one when it can't be loaded.
pub fn load(theme: &str, aou_mod: &str) -> Palette {
    // (MAO_THEME overrides the setting, for pictures of the interface in a theme)
    let forced = std::env::var("MAO_THEME").ok();
    let theme = forced.as_deref().unwrap_or(theme);
    // (the built-in look has the game's named text colours all the same: the markup of the game's own texts names them)
    let built_in = || {
        let text = GAME.get().and_then(|g| mao_formats::files::read_to_string(g.join("cd_image/gui/Default/TextColors.xml")).ok());
        Palette { text_colors: text.map(|xml| text_colors(&xml)).unwrap_or_default(), ..Palette::built_in() }
    };
    let Some(dir) = theme_dir(theme, aou_mod) else { return built_in() };
    match Palette::from_dir(theme, &dir) {
        Some(p) => p,
        None => {
            eprintln!("theme {theme:?}: no GUIColors.xml in {} (using the built-in look)", dir.display());
            built_in()
        }
    }
}

/// The themes to offer: the built-in look, then each of the others whose files exist.
pub fn available(aou_mod: &str) -> Vec<String> {
    let mut out = vec!["built-in".to_string()];
    for name in std::iter::once("original").chain(AOU_THEMES.iter().map(|(n, _)| *n)) {
        if theme_dir(name, aou_mod).is_some_and(|d| mao_formats::files::exists(d.join("GUIColors.xml"))) {
            out.push(name.to_string());
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    const ORIGINAL: &str = r#"<?xml version="1.0" ?>
<Root>
    <Color name="Default"      color="0x0080e9f3" />
    <Color name="Selected"     color="0x00ffffcc" />
    <Color name="Hover"        color="0x00a5ffdb" />
    <Color name="Text"         color="0x0099ccaa" />
    <Color name="TextSelected" color="0x0099ccaa" />
</Root>"#;

    const TRANSLUCENT: &str = r#"<Root>
    <!-- AOU Translucent Theme -->
    <Color name="Default"      color="0x00051a2a" />
    <Color name="Selected"     color="0x0030d2ff" />
    <Color name="Hover"        color="0x0040e6ff" />
    <Color name="Text"         color="0x0030d2ff" />
    <Color name="TextSelected" color="0x00ffffff" />
</Root>"#;

    #[test]
    fn colours_parse() {
        assert_eq!(parse_color("0x0030d2ff"), Some([0x30, 0xd2, 0xff]));
        assert_eq!(parse_color("#ff7718"), Some([0xff, 0x77, 0x18]));
        assert_eq!(parse_color("teal"), None);
    }

    #[test]
    fn the_original_gui_colours() {
        let p = Palette::from_gui_colors("original", ORIGINAL).unwrap();
        // Light cyan at rest: borders in it, panels darkened from it.
        assert_eq!(p.accent, [0x80, 0xe9, 0xf3]);
        assert!(luminance(p.panel) < 0.25);
        assert_eq!(p.hover, [0xa5, 0xff, 0xdb]);
        assert_eq!(p.text, [0x99, 0xcc, 0xaa]);
    }

    #[test]
    fn the_aou_translucent_theme() {
        let p = Palette::from_gui_colors("aou_translucent", TRANSLUCENT).unwrap();
        // Dark navy panels with neon cyan borders and text.
        assert_eq!(p.panel, [0x05, 0x1a, 0x2a]);
        assert_eq!(p.accent, [0x30, 0xd2, 0xff]);
        assert_eq!(p.text_selected, [0xff, 0xff, 0xff]);
    }

    #[test]
    fn text_colours_resolve_names() {
        let xml = r#"<Root><HTMLColor color="0xFFFF00" name="yellow"/><HTMLColor color="yellow" name="ct_otell"/><HTMLColor color="0x00ffff" name="ct_itell"/><HTMLColor color="nothing" name="bad"/></Root>"#;
        let colors = text_colors(xml);
        assert_eq!(colors, vec![("yellow".to_string(), [255, 255, 0]), ("ct_otell".to_string(), [255, 255, 0]), ("ct_itell".to_string(), [0, 255, 255])]);
        let palette = Palette { text_colors: colors, ..Palette::built_in() };
        assert_eq!(palette.text("ct_itell"), Some([0, 255, 255]));
        assert_eq!(palette.text("ct_unknown"), None);
    }

    #[test]
    fn settings_name_the_folders() {
        assert_eq!(theme_dir("built-in", ""), None);
        // (the bundled skins before the AOU mod's folder)
        let aou = bundled_gui_dir().map_or(PathBuf::from("C:/mods/aou/Gui/AOU_CYT"), |d| d.join("AOU_CYT"));
        assert_eq!(theme_dir("aou_cyt", "C:/mods/aou"), Some(aou));
        assert_eq!(theme_dir("D:/my theme", ""), Some(PathBuf::from("D:/my theme")));
        // (the built-in look but for the game's named text colours, when its files are at hand)
        let plain = |p: Palette| Palette { text_colors: Vec::new(), ..p };
        assert_eq!(plain(load("built-in", "")), Palette::built_in());
        // An unreadable folder falls back to the built-in look.
        assert_eq!(plain(load("nowhere-at-all/xyz", "")), Palette::built_in());
    }
}

/// The game's folder, when `set_game_dir` was given one.
pub fn game_dir() -> Option<&'static Path> {
    GAME.get().map(PathBuf::as_path)
}
