//! The original client's own window layout for a character, read from its preference files, so the windows here start
//! where that player left them in the original. The files live under `%LOCALAPPDATA%/Funcom/Anarchy Online/<hash>/Anarchy
//! Online/Prefs/`: `Prefs.xml` at the top holds the display size (`DisplayWidth`/`DisplayHeight`, the frame the places
//! are in), `derived/Char<id>/` the character's own:
//!
//! - `Prefs.xml`: `<Archive name="<X>Config">` with `<Rect name="WindowFrame" value="Rect(l,t,r,b)"/>` for the windows that
//!   keep their own config (`SkillConfig`, `OptionWindowConfig`, `PerkWindowConfig`, `NPCChatWindowConfig`, …);
//! - `DockAreas/DockArea<n>.xml`: a `DockTabbedWindow` each, its `WindowFrame` and `docked_view_identities` (the window
//!   names `inventory_window`, `mission_window`, `info_window`, a backpack's `Container_….xml`);
//! - `Chat/Windows/Window<n>/Config.xml`: a chat window's `WindowFrame`, `name` and whether it is the startup window;
//! - `Containers/ShortcutBar_<n>.xml`: the shortcut bars' frames (not used: the bars here are the client's own).
//!
//! The rollup area's pages (wear, NCU, programs, team, faction: `DockableViewDockName="RollupArea"`) have no frame; they
//! stack down the right edge (`rollup.rs`). Frames are scaled from the original's display to this window's size, so a
//! layout made on another monitor keeps its proportions. `MAO_AO_PREFS=<Prefs folder>` names the folder outright.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

/// The original's frames for the windows here, by their `chrome::window` names, in the original's display's pixels.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Original {
    pub display: [f32; 2],
    pub frames: BTreeMap<String, [f32; 4]>,
}

/// The original's config and window names and the names the windows have here.
const NAMES: &[(&str, &str)] = &[
    ("inventory_window", "Inventory"),
    ("mission_window", "MissionView"),
    ("info_window", "Info"),
    ("friends_window", "Friends"),
    ("pet_window", "PetWindow"),
    ("SkillConfig", "Skills"),
    ("OptionWindowConfig", "Option panel"),
    ("PerkWindowConfig", "Perks"),
    ("NPCChatWindowConfig", "NPCChat"),
    ("ResearchConfig", "Research"),
    ("PetWindowConfig", "PetWindow"),
    ("LFTWindowConfig", "LFT"),
    ("FriendsWindowConfig", "Friends"),
    ("StatWindowConfig", "Stats"),
    ("KnowledgeViewConfig", "Knowledge"),
    ("MarketWindowConfig", "Browser"),
    ("TradeSkillViewConfig", "Tradeskill"),
    ("TeamViewConfig", "Team"),
    ("RaidConfig", "Raid"),
    ("MissionSelectionViewConfig", "Missions"),
];

/// The Prefs folder and the character folder in it: `MAO_AO_PREFS`, else each `%LOCALAPPDATA%/Funcom/Anarchy
/// Online/*/Anarchy Online/Prefs`; the character's own `derived/Char<id>` folder when the original has played it here, else
/// the player's most recently written one (their layout from another character, which is what they are used to).
pub fn prefs_dirs(character: u32) -> Option<(PathBuf, PathBuf)> {
    let roots: Vec<PathBuf> = match std::env::var("MAO_AO_PREFS") {
        Ok(dir) => vec![PathBuf::from(dir)],
        Err(_) => {
            let local = std::env::var("LOCALAPPDATA").ok()?;
            let base = Path::new(&local).join("Funcom/Anarchy Online");
            let mut hashes: Vec<PathBuf> = std::fs::read_dir(&base).ok()?.flatten().map(|e| e.path()).collect();
            hashes.sort();
            hashes.into_iter().map(|h| h.join("Anarchy Online/Prefs")).collect()
        }
    };
    let own = format!("Char{character}");
    if let Some(root) = roots.iter().find(|p| p.join("derived").join(&own).is_dir()) {
        return Some((root.clone(), root.join("derived").join(&own)));
    }
    let mut newest: Option<(std::time::SystemTime, PathBuf, PathBuf)> = None;
    for root in &roots {
        let Ok(entries) = std::fs::read_dir(root.join("derived")) else { continue };
        for e in entries.flatten() {
            let dir = e.path();
            if !dir.file_name().is_some_and(|n| n.to_string_lossy().starts_with("Char")) {
                continue;
            }
            let Ok(meta) = std::fs::metadata(dir.join("Prefs.xml")) else { continue };
            let Ok(written) = meta.modified() else { continue };
            if newest.as_ref().is_none_or(|(t, _, _)| written > *t) {
                newest = Some((written, root.clone(), dir));
            }
        }
    }
    newest.map(|(_, root, dir)| (root, dir))
}

/// A `Rect(l,t,r,b)` value.
pub fn rect(value: &str) -> Option<[f32; 4]> {
    let inner = value.trim().strip_prefix("Rect(")?.strip_suffix(')')?;
    let v: Vec<f32> = inner.split(',').filter_map(|n| n.trim().parse().ok()).collect();
    (v.len() == 4 && v[2] > v[0] && v[3] > v[1]).then(|| [v[0], v[1], v[2], v[3]])
}

/// The `WindowFrame` rect of an archive's text.
fn window_frame(text: &str) -> Option<[f32; 4]> {
    rect(value(text, "WindowFrame")?)
}

/// The `<Value name="X" value="…"/>` values of a preferences file.
fn value<'a>(text: &'a str, name: &str) -> Option<&'a str> {
    let key = format!("name=\"{name}\"");
    let at = text.find(&key)?;
    let rest = &text[at..];
    // (a string value is in single quotes: `value='&quot;inventory_window&quot;'`)
    let value = rest.find("value=")? + 6;
    let quote = rest[value..].chars().next()?;
    if quote != '"' && quote != '\'' {
        return None;
    }
    let start = value + 1;
    let end = rest[start..].find(quote)?;
    Some(&rest[start..start + end])
}

/// The named archives of a `Prefs.xml`: each `<Archive name="…" code="0">…</Archive>` at the top level and its frame.
pub fn config_frames(text: &str) -> Vec<(String, [f32; 4])> {
    let mut out = Vec::new();
    let mut rest = text;
    while let Some(at) = rest.find("<Archive name=\"") {
        let name_start = at + 15;
        let Some(name_len) = rest[name_start..].find('"') else { break };
        let name = &rest[name_start..name_start + name_len];
        let body_start = name_start + name_len;
        let Some(tag_end) = rest[body_start..].find('>') else { break };
        let self_closing = rest[body_start..body_start + tag_end].ends_with('/');
        if self_closing {
            rest = &rest[body_start + tag_end..];
            continue;
        }
        // (the archive ends at the first `</Archive>` after its last nested archive: count the nesting)
        let mut depth = 1;
        let mut pos = body_start + tag_end;
        let mut end = None;
        while depth > 0 {
            let open = rest[pos..].find("<Archive ");
            let close = rest[pos..].find("</Archive>");
            match (open, close) {
                (Some(o), Some(c)) if o < c => {
                    let tag_end = rest[pos + o..].find('>').unwrap_or(0);
                    if !rest[pos + o..pos + o + tag_end].ends_with('/') {
                        depth += 1;
                    }
                    pos += o + tag_end.max(1);
                }
                (_, Some(c)) => {
                    depth -= 1;
                    pos += c + 10;
                    if depth == 0 {
                        end = Some(pos);
                    }
                }
                _ => break,
            }
        }
        let Some(end) = end else { break };
        let body = &rest[body_start..end];
        if let Some(frame) = window_frame(body) {
            out.push((name.to_string(), frame));
        }
        rest = &rest[end..];
    }
    out
}

/// A dock area file: the window identities it holds and its frame.
pub fn dock_area(text: &str) -> Option<(Vec<String>, [f32; 4])> {
    let ids = value(text, "docked_view_identities")?.replace("&quot;", "");
    let frame = window_frame(text)?;
    Some((ids.split(',').map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect(), frame))
}

/// Reads the character's layout: the display size and every frame for a window known here.
pub fn load(character: u32) -> Option<Original> {
    let (prefs, dir) = prefs_dirs(character)?;
    let mut out = Original::default();
    let top = std::fs::read_to_string(prefs.join("Prefs.xml")).unwrap_or_default();
    out.display = [value(&top, "DisplayWidth").and_then(|v| v.parse().ok()).unwrap_or(0.0), value(&top, "DisplayHeight").and_then(|v| v.parse().ok()).unwrap_or(0.0)];
    let here = |original: &str| NAMES.iter().find(|(o, _)| *o == original).map(|(_, h)| h.to_string());
    if let Ok(text) = std::fs::read_to_string(dir.join("Prefs.xml")) {
        for (name, frame) in config_frames(&text) {
            if let Some(h) = here(&name) {
                out.frames.insert(h, frame);
            }
        }
    }
    if let Ok(areas) = std::fs::read_dir(dir.join("DockAreas")) {
        for entry in areas.flatten() {
            let Ok(text) = std::fs::read_to_string(entry.path()) else { continue };
            if let Some((ids, frame)) = dock_area(&text) {
                for id in ids {
                    if let Some(h) = here(&id) {
                        out.frames.insert(h, frame);
                    }
                }
            }
        }
    }
    // (the chat windows: the startup window is the client's "Chat")
    if let Ok(windows) = std::fs::read_dir(dir.join("Chat/Windows")) {
        let mut found: Vec<(bool, [f32; 4])> = Vec::new();
        for entry in windows.flatten() {
            let Ok(text) = std::fs::read_to_string(entry.path().join("Config.xml")) else { continue };
            if let Some(frame) = window_frame(&text) {
                found.push((value(&text, "is_startup_window") == Some("true"), frame));
            }
        }
        if let Some((_, frame)) = found.iter().find(|(startup, _)| *startup).or(found.first()) {
            out.frames.insert("Chat".to_string(), *frame);
        }
    }
    (!out.frames.is_empty()).then_some(out)
}

impl Original {
    /// A window's frame scaled to a display of `size` (this window's), as left, top, width and height; the original's
    /// frame as it is when the displays are the same size or the original's is unknown.
    pub fn frame(&self, name: &str, size: [f32; 2]) -> Option<[f32; 4]> {
        let f = self.frames.get(name)?;
        let (sx, sy) = if self.display[0] > 0.0 && self.display[1] > 0.0 { (size[0] / self.display[0], size[1] / self.display[1]) } else { (1.0, 1.0) };
        Some([f[0] * sx, f[1] * sy, (f[2] - f[0]) * sx, (f[3] - f[1]) * sy])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frames_are_read_from_the_archives_and_dock_areas() {
        let prefs = r#"<Root>
    <Archive name="BattlestationConfig" code="0" />
    <Archive name="CCHealthBarConfig" code="0">
        <Rect name="WindowFrame" value="Rect(0.000000,0.000000,9.000000,134.000000)" />
    </Archive>
    <Archive name="NCUWindowConfig" code="0">
        <Archive code="0" name="listview_config">
            <Int32 name="ncu_listview_flags" value="0" />
        </Archive>
    </Archive>
    <Archive name="SkillConfig" code="0">
        <Rect name="WindowFrame" value="Rect(200.000000,180.000000,896.000000,718.000000)" />
    </Archive>
</Root>"#;
        let frames = config_frames(prefs);
        assert_eq!(frames, vec![("CCHealthBarConfig".to_string(), [0.0, 0.0, 9.0, 134.0]), ("SkillConfig".to_string(), [200.0, 180.0, 896.0, 718.0])]);
        let area = r#"<Archive code="0">
    <Archive code="0" name="dock_config">
        <Archive code="0" name="dock_node_configs" />
        <String name="docked_view_identities" value='&quot;inventory_window&quot;' />
        <Bool name="WindowPinButtonState" value="false" />
        <Rect name="WindowFrame" value="Rect(2709.000000,1233.000000,3290.000000,1408.000000)" />
        <Int32 name="selected_tab" value="0" />
    </Archive>
    <String name="dock_type" value='&quot;DockTabbedWindow&quot;' />
</Archive>"#;
        assert_eq!(dock_area(area), Some((vec!["inventory_window".to_string()], [2709.0, 1233.0, 3290.0, 1408.0])));
        assert_eq!(rect("Rect(1,2,0,5)"), None);
    }

    #[test]
    fn frames_scale_to_this_display() {
        let mut o = Original { display: [5120.0, 1440.0], frames: BTreeMap::new() };
        o.frames.insert("Chat".into(), [1470.0, 1212.0, 2402.0, 1409.0]);
        assert_eq!(o.frame("Chat", [5120.0, 1440.0]), Some([1470.0, 1212.0, 932.0, 197.0]));
        let half = o.frame("Chat", [2560.0, 720.0]).unwrap();
        assert_eq!(half, [735.0, 606.0, 466.0, 98.5]);
        assert_eq!(o.frame("Skills", [2560.0, 720.0]), None);
    }
}
