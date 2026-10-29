//! `DistributedValue_c` (Utils.dll): the named values the interface is driven by — which windows are open
//! (`inventory_window`, `skill_window`, ...), which parts of the control center show (`cc_section1`, `cc_left_wing`, ...),
//! the options — and the criteria (`activate_criteria`, `criteria`) the views and menus are shown by, which read them.
//!
//! The values load as GUI.dll's FUN_1006cc55 loads them: the game's `Gui/Default/Variables.xml`, `MainPrefs.xml`,
//! `LoginPrefs.xml` and `CharPrefs.xml`, then the skin's own four files over them (`LoadConfig`: each `<Value name=...
//! value=...>` read with `Variant::LoadFromString`; `<Archive>` entries are not kept here). The user's `Prefs.xml` is the
//! client's own settings.
//!
//! An expression's variables are the program's criteria context (anarchyonline.exe FUN_00408268): `stat:<name>` or
//! `s:<name>` the player's stat (named as `N3Msg_StringToStat` names them, [`super::stat_names`]); `dvalue:`, `m:`,
//! `module:`, `mcfg:`, `lcfg:` and `ccfg:` a distributed value; anything else a value the caller adds (a menu entry's
//! `item_count`). A name none of them knows (an unknown stat, a value never set: the context prints "Error: could not
//! find variable" and declines) goes on to the parser's constants, which have none of these: "Unknown variable" and 0.

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::path::Path;

use super::expression::{self, Host, State};
use super::variant::Variant;

/// `IndependentPrefs_t`'s preferences as GUI.dll's `Preferences_t::SetDefaultCharPrefs` (0x10124951) and
/// `SetDefaultLoginPrefs` (0x1012500a) start them (`InitDefaultInt` / `InitDefaultFloat`): name, a float, the default,
/// the least and the most. The option panel's `opt_type="int"`/`"float"` settings are these.
pub const INDEPENDENT_PREFS: &[(&str, bool, f64, f64, f64)] = &[
    ("IsFirstTime", false, 1.0, 0.0, 1.0),
    ("ShowDropItemDialog", false, 1.0, 0.0, 1.0),
    ("IsOrgNameShownOverHead", false, 1.0, 0.0, 1.0),
    ("IsSpaceShipsShown", false, 1.0, 0.0, 1.0),
    ("BuildMenuX", false, 0.0, 0.0, 9999.0),
    ("BuildMenuY", false, 0.0, 0.0, 9999.0),
    ("BuildMenuCX", false, 0.0, 0.0, 1000.0),
    ("BuildMenuCY", false, 0.0, 0.0, 1000.0),
    ("IsFactionTitleShown", false, 1.0, 0.0, 1.0),
    ("WaitForVertSync", false, 0.0, 0.0, 1.0),
    ("MouseLagFix", false, 0.0, 0.0, 1.0),
    ("UseOffscreenSurfaceTechnology", false, 0.0, 0.0, 1.0),
    ("FadeCharacter", false, 1.0, 0.0, 1.0),
    ("FadeCharacterStartDist", true, 1.5, 0.1, 10.0),
    ("FadeCharacterEndDist", true, 0.7, 0.1, 10.0),
    ("FadeCharacterEndAlpha", true, 0.15, 0.0, 1.0),
    ("WasCharacterCreated", false, 1.0, 0.0, 1.0),
    ("CCSelectedBreed", false, 0.0, 0.0, 7.0),
    ("CCSelectedHeight", false, 0.0, 0.0, 3.0),
    ("CCSelectedSize", false, 0.0, 0.0, 3.0),
    ("CCSelectedHead", false, 0.0, 0.0, 300.0),
    ("CCSelectedProfession", false, 0.0, 0.0, 14.0),
    ("PreferredCameraMode", false, 3.0, 0.0, 3.0),
    ("UseNoBobCamera", false, 0.0, 0.0, 1.0),
    ("IsChatHidden", false, 0.0, 0.0, 1.0),
    ("StarRotation", false, 1.0, 0.0, 1.0),
    ("RealisticMoons", false, 1.0, 0.0, 1.0),
    ("UseWindEngine", false, 1.0, 0.0, 1.0),
    ("UseCloudShadows", false, 1.0, 0.0, 1.0),
    ("UseParticleWeightPhysics", false, 1.0, 0.0, 1.0),
    ("UseStormParticles", false, 1.0, 0.0, 1.0),
    ("RealisticWater", false, 1.0, 0.0, 1.0),
    ("Wildlife", false, 1.0, 0.0, 1.0),
    ("RealisticClouds", false, 1.0, 0.0, 1.0),
    ("SimpleClouds", false, 1.0, 0.0, 1.0),
    ("Shadows", false, 1.0, 0.0, 1.0),
    ("BuffsFX", false, 1.0, 0.0, 1.0),
    ("TracersFX", false, 1.0, 0.0, 1.0),
    ("NanoEffectFX", false, 1.0, 0.0, 1.0),
    ("MuzzleFlashFX", false, 1.0, 0.0, 1.0),
    ("EnvironmentFX", false, 1.0, 0.0, 1.0),
    ("OthersFX", false, 1.0, 0.0, 1.0),
    ("ShowAllNames", false, 1.0, 0.0, 1.0),
    ("SmoothAnimations", false, 1.0, 0.0, 1.0),
    ("ShowIPPerkbutton", false, 1.0, 0.0, 1.0),
    ("ChatIndicator", false, 1.0, 0.0, 1.0),
    ("3rdPersonCamera", false, 1.0, 0.0, 1.0),
    ("ShowNPCQuestions", false, 1.0, 0.0, 1.0),
    ("MouseLookInverted", false, 0.0, 0.0, 1.0),
    ("FogMode", false, 3.0, 0.0, 3.0),
    ("GroundRendering", false, 3.0, 0.0, 3.0),
    ("ViewDistance", true, 0.8, 0.0, 1.0),
];

#[derive(Default)]
pub struct Store {
    values: BTreeMap<String, Variant>,
    /// A value's range (`min`/`max` of its `<Value>`, or its preference's), for the option panel's sliders.
    ranges: BTreeMap<String, (f64, f64)>,
    /// The names set since [`Store::take_changed`] (`SetDValue`'s observers).
    changed: Vec<String>,
    /// The names the player changed (written to their preferences file).
    user: std::collections::BTreeSet<String>,
}

impl Store {
    /// The values GUI.dll loads for a skin (FUN_1006cc55): the game's defaults, then the skin's files.
    pub fn load(game_default: Option<&Path>, skin: Option<&Path>) -> Store {
        let mut s = Store::default();
        for dir in [game_default, skin].into_iter().flatten() {
            for file in ["Variables.xml", "MainPrefs.xml", "LoginPrefs.xml", "CharPrefs.xml"] {
                if let Ok(text) = std::fs::read(dir.join(file)) {
                    s.load_config(&String::from_utf8_lossy(&text));
                }
            }
        }
        for &(name, float, default, lo, hi) in INDEPENDENT_PREFS {
            s.values.entry(name.to_string()).or_insert(if float { Variant::Float(default as f32) } else { Variant::Int32(default as i32) });
            s.ranges.entry(name.to_string()).or_insert((lo, hi));
        }
        s.changed.clear();
        s
    }

    /// The player's own values over the defaults: a file of `<Value>`s as the program saves its `Prefs.xml`.
    pub fn load_user(&mut self, path: &Path) {
        let Ok(text) = std::fs::read_to_string(path) else { return };
        let before: Vec<String> = self.values.keys().cloned().collect();
        self.load_config(&text);
        for root in super::xml::parse(&text) {
            for e in root.children.iter().filter(|e| e.name == "Value") {
                if let Some(n) = e.attribute("name") {
                    self.user.insert(n.to_string());
                }
            }
        }
        let _ = before;
        self.changed.clear();
    }

    /// Writes the values the player changed (`DistributedValue_c::SaveConfig`'s form: `<Value name=".." value=".."/>`).
    pub fn save_user(&self, path: &Path) {
        let mut out = String::from("<Root>\n");
        for name in &self.user {
            let Some(v) = self.values.get(name) else { continue };
            let text = match v {
                Variant::String(s) => format!("&quot;{}&quot;", String::from_utf8_lossy(s).replace('&', "&amp;").replace('<', "&lt;").replace('"', "&quot;")),
                Variant::Float(f) => format!("{f:.6}"),
                Variant::Double(d) => format!("{d:.6}"),
                other => String::from_utf8_lossy(&other.as_string()).into_owned(),
            };
            out += &format!("    <Value name=\"{name}\" value='{text}' />\n");
        }
        out += "</Root>\n";
        let _ = std::fs::write(path, out);
    }

    /// A value's range, if it has one.
    pub fn range(&self, name: &str) -> Option<(f64, f64)> {
        self.ranges.get(name).copied()
    }

    /// A value the player set (in the option panel): set, and kept in their preferences.
    pub fn set_user(&mut self, name: &str, value: Variant) {
        self.user.insert(name.to_string());
        self.set(name, value);
    }

    /// `LoadConfig`: each `<Value name="..." value="..."/>` of the file's root.
    pub fn load_config(&mut self, text: &str) {
        for root in super::xml::parse(text) {
            for e in root.children.iter().filter(|e| e.name == "Value") {
                let (Some(name), Some(value)) = (e.attribute("name"), e.attribute("value")) else { continue };
                let mut v = Variant::new();
                if v.load_from_string(value.as_bytes()) {
                    self.values.insert(name.to_string(), v);
                }
                let bound = |n: &str| e.attribute(n).and_then(|t| t.trim().parse::<f64>().ok());
                if let (Some(lo), Some(hi)) = (bound("min"), bound("max")) {
                    self.ranges.insert(name.to_string(), (lo, hi));
                }
            }
        }
    }

    /// `DoesVariableExist` / `GetDValue`.
    pub fn get(&self, name: &str) -> Option<&Variant> {
        self.values.get(name)
    }

    pub fn get_bool(&self, name: &str) -> bool {
        self.values.get(name).is_some_and(Variant::as_bool)
    }

    /// `SetDValue`: the value, noting the change when it differs.
    pub fn set(&mut self, name: &str, value: Variant) {
        if self.values.get(name) != Some(&value) {
            self.values.insert(name.to_string(), value);
            self.changed.push(name.to_string());
        }
    }

    pub fn take_changed(&mut self) -> Vec<String> {
        std::mem::take(&mut self.changed)
    }

    /// `ExpressionParser_c::ParseExpression` with the program's context over these values and the player's stats, and
    /// `extra` for the caller's own names: the value, `None` when the text is not an expression.
    pub fn eval(&self, text: &str, stats: &BTreeMap<u32, i64>, extra: &dyn Fn(&str) -> Option<Variant>) -> Option<Variant> {
        thread_local! {
            static STATE: RefCell<State> = RefCell::new(State::new());
        }
        struct Ctx<'a> {
            store: &'a Store,
            stats: &'a BTreeMap<u32, i64>,
            extra: &'a dyn Fn(&str) -> Option<Variant>,
        }
        impl Host for Ctx<'_> {
            fn variable(&mut self, name: &[u8]) -> Variant {
                let name = String::from_utf8_lossy(name.split(|&c| c == 0).next().unwrap_or(&[])).into_owned();
                let lower = name.to_ascii_lowercase();
                let rest = |p: &str| lower.starts_with(p).then(|| &name[p.len()..]);
                if let Some(stat) = rest("stat:").or_else(|| rest("s:")) {
                    if let Some(&(k, _)) = super::stat_names::STAT_NAMES.iter().find(|(_, n)| n.eq_ignore_ascii_case(stat)) {
                        return Variant::from(self.stats.get(&k).copied().unwrap_or(0) as i32);
                    }
                    return Variant::from(0);
                }
                if let Some(v) = ["module:", "dvalue:", "m:", "mcfg:", "lcfg:", "ccfg:"].iter().find_map(|p| rest(p)) {
                    return self.store.get(v).cloned().unwrap_or(Variant::from(0));
                }
                if let Some(v) = (self.extra)(&name) {
                    return v;
                }
                Variant::from(0)
            }
            fn print(&mut self, _line: &[u8]) {}
            fn fatal(&mut self, message: &str) -> ! {
                panic!("{message}")
            }
        }
        let mut ctx = Ctx { store: self, stats, extra };
        STATE.with(|state| {
            let ok = expression::parse(state, text.as_bytes(), &mut ctx);
            ok.then(|| state.borrow().result.clone())
        })
    }

    /// A criteria string as a `CriteriaMonitor_c` holds it: true when it parses and its value is true; an empty one is
    /// true.
    pub fn criteria(&self, text: &str, stats: &BTreeMap<u32, i64>, extra: &dyn Fn(&str) -> Option<Variant>) -> bool {
        text.trim().is_empty() || self.eval(text, stats, extra).is_some_and(|v| v.as_bool())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn criteria_read_values_and_stats() {
        let mut s = Store::default();
        s.load_config(r#"<Root><Value name="cc_section1" value="true"/><Value name="cc_menu_pos" value="0"/><Value name="Font" value='&quot;Verdana&quot;'/></Root>"#);
        let stats = BTreeMap::from([(54u32, 12i64), (389, 0x2)]);
        let none = |_: &str| None;
        assert!(s.criteria("dvalue:cc_section1 && dvalue:cc_menu_pos==0", &stats, &none));
        assert!(!s.criteria("dvalue:cc_section1 && dvalue:nothing", &stats, &none));
        assert!(s.criteria("dvalue:nothing==0", &stats, &none));
        assert!(s.criteria("stat:level>=10 && ((stat:expansion & 0x2) || (stat:expansion & 0x8))", &stats, &none));
        assert!(!s.criteria("s:level<10", &stats, &none));
        assert_eq!(s.get("Font").map(|v| v.as_string()), Some(b"Verdana".to_vec()));
        s.set("inventory_window", Variant::from(true));
        assert!(s.eval("dvalue:inventory_window==false", &stats, &none).is_some_and(|v| !v.as_bool()));
        assert_eq!(s.take_changed(), vec!["inventory_window".to_string()]);
        s.load_config(r#"<Root><Value max="1.0" min="0.0" name="MasterVolume" value="1.0"/></Root>"#);
        assert_eq!(s.range("MasterVolume"), Some((0.0, 1.0)));
        // the player's own values round trip
        s.set_user("MasterVolume", Variant::Float(0.25));
        s.set_user("Font", Variant::from("Arial"));
        let path = std::env::temp_dir().join(format!("mao-prefs-{}.xml", std::process::id()));
        s.save_user(&path);
        let mut t = Store::default();
        t.load_user(&path);
        let _ = std::fs::remove_file(&path);
        assert_eq!(t.get("MasterVolume").map(|v| v.as_float()), Some(0.25));
        assert_eq!(t.get("Font").map(|v| v.as_string()), Some(b"Arial".to_vec()));
        let extra = |n: &str| (n == "active_item_count").then(|| Variant::from(3));
        assert!(s.criteria("active_item_count>0", &stats, &extra));
    }
}
