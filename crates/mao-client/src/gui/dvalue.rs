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

#[derive(Default)]
pub struct Store {
    values: BTreeMap<String, Variant>,
    /// The names set since [`Store::take_changed`] (`SetDValue`'s observers).
    changed: Vec<String>,
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
        s.changed.clear();
        s
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
        let extra = |n: &str| (n == "active_item_count").then(|| Variant::from(3));
        assert!(s.criteria("active_item_count>0", &stats, &extra));
    }
}
