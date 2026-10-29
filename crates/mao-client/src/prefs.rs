//! The player's preferences as the rest of the client reads them: a copy of the settings store's numbers (the option
//! panel's `IndependentPrefs_t` preferences and distributed values, `gui::dvalue`), kept by the control center when it
//! loads and whenever one changes. A preference never set reads as GUI.dll's default
//! ([`crate::gui::dvalue::INDEPENDENT_PREFS`]).
//!
//! The effect categories are Gamecode's `_EffectHandler_t` ones (its constructor 0x100ce74e builds the mask from
//! DisplaySystem's `EnvironmentPreferences_t` flags; `isEffectInCategoryActive` 0x100cdf48 drops an effect of a category
//! switched off): [`BUFFS`] (`BuffsFX`), [`TRACERS`] (`TracersFX`), [`NANO`] (`NanoEffectFX`), [`MUZZLE`]
//! (`MuzzleFlashFX`), [`ENVIRONMENT`] (`EnvironmentFX`), [`OTHERS`] (`OthersFX`).

use std::collections::BTreeMap;
use std::sync::RwLock;

static PREFS: RwLock<BTreeMap<String, f64>> = RwLock::new(BTreeMap::new());

pub const BUFFS: u32 = 1;
pub const TRACERS: u32 = 2;
pub const NANO: u32 = 4;
pub const MUZZLE: u32 = 8;
pub const ENVIRONMENT: u32 = 0x10;
pub const OTHERS: u32 = 0x20;

/// A preference's value now.
pub fn set(name: &str, value: f64) {
    if let Ok(mut p) = PREFS.write() {
        p.insert(name.to_string(), value);
    }
}

/// A preference: the value set, else GUI.dll's default, else `None`.
pub fn get(name: &str) -> Option<f64> {
    if let Some(v) = PREFS.read().ok().and_then(|p| p.get(name).copied()) {
        return Some(v);
    }
    crate::gui::dvalue::INDEPENDENT_PREFS.iter().find(|p| p.0 == name).map(|p| p.2)
}

/// A switch: on unless set to 0 (or its default is 0).
pub fn on(name: &str) -> bool {
    get(name).is_none_or(|v| v != 0.0)
}

/// `isEffectInCategoryActive`: whether effects of a category are shown.
pub fn effect_category_active(category: u32) -> bool {
    let mask = [("BuffsFX", BUFFS), ("TracersFX", TRACERS), ("NanoEffectFX", NANO), ("MuzzleFlashFX", MUZZLE), ("EnvironmentFX", ENVIRONMENT), ("OthersFX", OTHERS)]
        .into_iter()
        .filter(|(n, _)| on(n))
        .fold(0, |m, (_, bit)| m | bit);
    category == 0 || mask & category != 0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn categories_follow_their_switches() {
        assert!(effect_category_active(NANO));
        set("NanoEffectFX", 0.0);
        assert!(!effect_category_active(NANO));
        assert!(effect_category_active(BUFFS) && effect_category_active(0));
        set("NanoEffectFX", 1.0);
        // (a preference never set reads as its default)
        assert_eq!(get("FogMode"), Some(3.0));
        assert!(!on("MouseLookInverted"));
    }
}
