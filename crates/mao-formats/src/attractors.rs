//! The effects a mesh carries on its connectors (the "attractor effects"): DisplaySystem's `VisualMesh_t` set-up
//! (FUN_1006d49a; `make_attractors` in the drop-in's `asyncmesh.rs`) goes over the mesh's connectors and makes an
//! `AttractorEffectData_t` for each whose name starts with `eff_`: a fire on a campfire's anchor, a halo on a lamp's.
//! The name after the prefix picks the effect's template, and for the table's prefix entries the hex digits that follow
//! give a colour and a radius.

/// An effect a connector names.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Effect {
    /// The effect's template.
    pub id: i32,
    /// A colour of the effect's own (four values from 0 to 1, as the name's first four hex bytes over 255), when the
    /// name has four or more.
    pub colour: Option<[f32; 4]>,
    /// A radius (the name's last hex byte over 255, times 4), when the name has one hex byte or five.
    pub radius: Option<f32>,
}

/// The table at FUN_1006d49a: the name after `eff_` and the template; an entry ending in `*` is a prefix, the others
/// are whole names. In the original's order, which matters (`fire02` stands before `fire0*`).
const EFFECTS: [(&str, i32); 31] = [
    ("muzzleflash", 0x7d0),
    ("explosion", 0x834),
    ("fire02", 0x898),
    ("fire0*", 0x898),
    ("fire1*", 0x899),
    ("fire2*", 0x89a),
    ("fire3*", 0x89b),
    ("fire4*", 0x89c),
    ("fire5*", 0x89d),
    ("fire6*", 0x89e),
    ("fire7*", 0x89f),
    ("fire8*", 0x8a0),
    ("fire9*", 0x8a1),
    ("smoke1", 0x8fd),
    ("smoke2", 0x8fe),
    ("smoke*", 0x8fc),
    ("halo_white", 0x1b58),
    ("halo_red", 0x1b59),
    ("halo_green", 0x1b5a),
    ("halo_blue", 0x1b5b),
    ("billboard", 0x1bbc),
    ("geyshir*", 0x2f44),
    ("waterdrip*", 0x2f4e),
    ("halo_S*", 0x1b5e),
    ("torch", 0x2f5c),
    ("lightsabre*", 0x15f90),
    ("halo_M*", 0x1b5c),
    ("halo_L*", 0x1b5d),
    ("haloS*", 0x1b5e),
    ("haloM*", 0x1b5c),
    ("haloL*", 0x1b5d),
];

/// The red halo: what a name the table does not have gets.
pub const UNKNOWN: i32 = 0x1b59;

/// `sscanf("%d")`'s number at the head of a text (none: 0, as the original's variable starts).
fn leading_number(text: &str) -> i32 {
    let text = text.trim_start();
    let end = text.char_indices().find(|&(i, c)| !(c.is_ascii_digit() || (i == 0 && (c == '-' || c == '+')))).map_or(text.len(), |(i, _)| i);
    text[..end].parse().unwrap_or(0)
}

/// The effect a connector's name stands for: none for a name without the `eff_` prefix, for the rendering effects
/// (`eff_hologram`, `eff_space`, `eff_clear...`, which change how the mesh itself is drawn) and for a universal effect
/// whose number is 0.
pub fn effect_of(name: &str) -> Option<Effect> {
    let rest = name.strip_prefix("eff_")?;
    let plain = |id: i32| (id != 0).then_some(Effect { id, colour: None, radius: None });
    if let Some(number) = rest.strip_prefix("universal_") {
        return plain(leading_number(number));
    }
    if let Some(number) = rest.strip_prefix("universal") {
        return plain(leading_number(number));
    }
    if rest.starts_with("hologram") || rest.starts_with("space") || rest.starts_with("clear") {
        return None;
    }
    for (key, id) in EFFECTS {
        match key.strip_suffix('*') {
            Some(prefix) => {
                let Some(digits) = rest.strip_prefix(prefix) else { continue };
                // (up to five hex bytes, two digits each; a lone last digit is a byte of its own. The original's `sscanf`
                // leaves a byte that is not hex at the one before it; here the reading stops there.)
                let bytes = digits.as_bytes();
                let mut values = Vec::new();
                let mut at = 0;
                while at < bytes.len() && values.len() < 5 {
                    let end = (at + 2).min(bytes.len());
                    let Some(v) = std::str::from_utf8(&bytes[at..end]).ok().and_then(|t| u8::from_str_radix(t, 16).ok()) else { break };
                    values.push(v as f32 / 255.0);
                    at = end;
                }
                let radius = matches!(values.len(), 1 | 5).then(|| values[values.len() - 1] * 4.0);
                let colour = (values.len() > 3).then(|| [values[0], values[1], values[2], values[3]]);
                return Some(Effect { id, colour, radius });
            }
            None if rest == key => return plain(id),
            None => {}
        }
    }
    plain(UNKNOWN)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_pick_their_effects() {
        let plain = |id| Some(Effect { id, colour: None, radius: None });
        assert_eq!(effect_of("eff_fire1"), plain(0x899));
        assert_eq!(effect_of("eff_fire02"), plain(0x898));
        assert_eq!(effect_of("eff_fire0"), plain(0x898));
        assert_eq!(effect_of("eff_smoke1"), plain(0x8fd));
        assert_eq!(effect_of("eff_smoke"), plain(0x8fc));
        assert_eq!(effect_of("eff_halo_white"), plain(0x1b58));
        assert_eq!(effect_of("eff_torch"), plain(0x2f5c));
        assert_eq!(effect_of("eff_universal_12200"), plain(12200));
        assert_eq!(effect_of("eff_universal12301"), plain(12301));
        // (a name the table lacks is the red halo; a whole-name entry with more after it is one of those)
        assert_eq!(effect_of("eff_something"), plain(UNKNOWN));
        assert_eq!(effect_of("eff_halo_whiter"), plain(UNKNOWN));
        // (not effects of this kind)
        assert_eq!(effect_of("eff_hologram"), None);
        assert_eq!(effect_of("eff_space01"), None);
        assert_eq!(effect_of("eff_universal_0"), None);
        assert_eq!(effect_of("attractor01"), None);
        assert_eq!(effect_of("fire1"), None);
    }

    #[test]
    fn hex_digits_give_a_colour_and_a_radius() {
        // one byte: a radius
        let e = effect_of("eff_halo_S80").unwrap();
        assert_eq!((e.id, e.colour), (0x1b5e, None));
        assert!((e.radius.unwrap() - 128.0 / 255.0 * 4.0).abs() < 1e-6);
        // four: a colour
        let e = effect_of("eff_haloMff8000ff").unwrap();
        assert_eq!((e.id, e.radius), (0x1b5c, None));
        assert_eq!(e.colour, Some([1.0, 128.0 / 255.0, 0.0, 1.0]));
        // five: both
        let e = effect_of("eff_halo_Lff0000ff40").unwrap();
        assert_eq!(e.colour, Some([1.0, 0.0, 0.0, 1.0]));
        assert!((e.radius.unwrap() - 64.0 / 255.0 * 4.0).abs() < 1e-6);
        // two or three: neither
        let e = effect_of("eff_halo_S8080").unwrap();
        assert_eq!((e.colour, e.radius), (None, None));
    }
}
