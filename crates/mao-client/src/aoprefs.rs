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
//! - `Containers/ShortcutBar_<n>.xml` (n from 0): the shortcut bars' frames, as `ShortcutBar_<n>` here (the original's bars,
//!   `shortcutbar.rs`, start there until moved).
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
    ("NotificationWindowConfig", "NotificationWindow"),
];

/// The `Prefs` folders of the original's installs: `MAO_AO_PREFS`, else each `%LOCALAPPDATA%/Funcom/Anarchy
/// Online/*/Anarchy Online/Prefs`.
fn prefs_roots() -> Option<Vec<PathBuf>> {
    Some(match std::env::var("MAO_AO_PREFS") {
        Ok(dir) => vec![PathBuf::from(dir)],
        Err(_) => {
            let local = std::env::var("LOCALAPPDATA").ok()?;
            let base = Path::new(&local).join("Funcom/Anarchy Online");
            let mut hashes: Vec<PathBuf> = std::fs::read_dir(&base).ok()?.flatten().map(|e| e.path()).collect();
            hashes.sort();
            hashes.into_iter().map(|h| h.join("Anarchy Online/Prefs")).collect()
        }
    })
}

/// The Prefs folder and the character's own `derived/Char<id>` folder in it, when the original has played the
/// character here; none otherwise. (The original reads a character's window places from that character's own
/// files, `ControlCenterModule_c::LoadUserConfig` 0x1006c3b1, and a character it never played has none: the shipped
/// `CharPrefs.xml` holds no frames, so its windows start at their views' own places. No other character's layout
/// stands in.)
pub fn prefs_dirs(character: u32) -> Option<(PathBuf, PathBuf)> {
    prefs_dirs_in(&prefs_roots()?, character)
}

/// [`prefs_dirs`] under these `Prefs` folders.
pub fn prefs_dirs_in(roots: &[PathBuf], character: u32) -> Option<(PathBuf, PathBuf)> {
    let own = format!("Char{character}");
    roots.iter().find(|p| p.join("derived").join(&own).is_dir()).map(|root| (root.clone(), root.join("derived").join(&own)))
}

/// The Prefs folder the player's original client wrote last (the one holding the most recently written character
/// file), for what is the install's and not a character's: its `scripts` folder beside it.
pub fn newest_prefs_root() -> Option<PathBuf> {
    let mut newest: Option<(std::time::SystemTime, PathBuf)> = None;
    for root in prefs_roots()? {
        let Ok(entries) = std::fs::read_dir(root.join("derived")) else { continue };
        for e in entries.flatten() {
            let dir = e.path();
            if !dir.file_name().is_some_and(|n| n.to_string_lossy().starts_with("Char")) {
                continue;
            }
            let Ok(meta) = std::fs::metadata(dir.join("Prefs.xml")) else { continue };
            let Ok(written) = meta.modified() else { continue };
            if newest.as_ref().is_none_or(|(t, _)| written > *t) {
                newest = Some((written, root.clone()));
            }
        }
    }
    newest.map(|(_, root)| root)
}

fn folders(dir: &Path) -> Vec<PathBuf> {
    let mut found: Vec<PathBuf> = std::fs::read_dir(dir).map(|entries| entries.flatten().map(|e| e.path()).filter(|p| p.is_dir()).collect()).unwrap_or_default();
    found.sort();
    found
}

/// The character's own `Prefs.xml` as the original keeps it, `<Prefs>/<user name>/Char<id>/Prefs.xml`
/// (`ControlCenterModule_c::LoadUserConfig` 0x1006c3b1; the user's folder is an account's name or `derived`), for
/// what has to be this character's and no other's: its keys (`keybindings.rs`). Looked for under `MAO_AO_PREFS`,
/// else under every `%LOCALAPPDATA%/Funcom/Anarchy Online/*/*/Prefs`; where more than one install has the
/// character, the file written last. None when the original has not played the character on this machine: no
/// other character's file stands in.
pub fn character_prefs(character: u32) -> Option<PathBuf> {
    let roots: Vec<PathBuf> = match std::env::var("MAO_AO_PREFS") {
        Ok(dir) => vec![PathBuf::from(dir)],
        Err(_) => {
            let local = std::env::var("LOCALAPPDATA").ok()?;
            folders(&Path::new(&local).join("Funcom/Anarchy Online")).iter().flat_map(|hash| folders(hash)).map(|install| install.join("Prefs")).collect()
        }
    };
    character_prefs_in(&roots, character)
}

/// [`character_prefs`] under these `Prefs` folders.
pub fn character_prefs_in(roots: &[PathBuf], character: u32) -> Option<PathBuf> {
    let own = format!("Char{character}");
    roots
        .iter()
        .flat_map(|root| folders(root))
        .map(|user| user.join(&own).join("Prefs.xml"))
        .filter_map(|file| Some((std::fs::metadata(&file).ok()?.modified().ok()?, file)))
        .max_by_key(|(written, _)| *written)
        .map(|(_, file)| file)
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

/// A `DockAreas/<name>.xml` file as the original writes it (`DockingController_c`, FUN_1003aad8): `dock_name`,
/// `dock_type` (`DockTabbedWindow` or `RollupController`) and the `dock_config` archive, which holds the views'
/// identities in their tabs' order (FUN_10039a07: one `docked_view_identities` string each, an `Array` of them when
/// there are several), the window's frame and `selected_tab` (-1: none).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct DockFile {
    pub name: String,
    pub kind: String,
    pub views: Vec<String>,
    pub selected: i32,
    pub frame: Option<[f32; 4]>,
}

pub fn dock_file(text: &str) -> Option<DockFile> {
    let unquote = |v: &str| v.replace("&quot;", "").trim().to_string();
    let mut views = Vec::new();
    let key = "name=\"docked_view_identities\"";
    let mut rest = text;
    while let Some(at) = rest.find(key) {
        // (the tag it is in: an `<Array …>` of `<String value=…/>`, or one `<String … value=…/>`)
        let tag = rest[..at].rfind('<').map_or("", |t| &rest[t..at]);
        if tag.starts_with("<Array") {
            let end = rest[at..].find("</Array>").map_or(rest.len(), |e| at + e);
            let mut body = &rest[at..end];
            while let Some(v) = body.find("value=") {
                let quoted = &body[v + 6..];
                let Some(quote) = quoted.chars().next().filter(|q| *q == '\'' || *q == '"') else { break };
                let Some(close) = quoted[1..].find(quote) else { break };
                views.push(unquote(&quoted[1..1 + close]));
                body = &quoted[1 + close..];
            }
            rest = &rest[end..];
        } else {
            if let Some(v) = value(&rest[at..], "docked_view_identities") {
                views.push(unquote(v));
            }
            rest = &rest[at + key.len()..];
        }
    }
    views.retain(|v| !v.is_empty());
    let name = value(text, "dock_name").map(unquote).unwrap_or_default();
    let kind = value(text, "dock_type").map(unquote).unwrap_or_default();
    (!name.is_empty() || !views.is_empty()).then(|| DockFile { name, kind, views, selected: value(text, "selected_tab").and_then(|v| v.parse().ok()).unwrap_or(-1), frame: window_frame(text) })
}

/// The character's tabbed dock areas (`DockTabbedWindow`s) in the original's files, for the windows' tab groups here
/// (`docks.rs`); none when the original has not played the character on this machine.
pub fn dock_files(character: u32) -> Vec<DockFile> {
    let Some((_, dir)) = prefs_dirs(character) else { return Vec::new() };
    let mut found: Vec<DockFile> = std::fs::read_dir(dir.join("DockAreas")).map(|areas| areas.flatten().filter_map(|e| std::fs::read_to_string(e.path()).ok()).filter_map(|t| dock_file(&t)).filter(|f| f.kind == "DockTabbedWindow").collect()).unwrap_or_default();
    found.sort_by(|a, b| a.name.cmp(&b.name));
    found
}

/// The rollup area's file (`DockAreas/RollupArea.xml`, dock type `RollupController`, written by FUN_10048e92): each
/// page's view identity with its `is_page_expanded` and `page_height` (the `dock_node_configs`, one archive a page in
/// the views' order).
pub fn rollup_pages(text: &str) -> Option<Vec<(String, bool, f32)>> {
    let file = dock_file(text).filter(|f| f.kind == "RollupController")?;
    let all = |name: &str| -> Vec<&str> {
        let key = format!("name=\"{name}\"");
        text.match_indices(&key).filter_map(|(at, _)| value(&text[at..], name)).collect()
    };
    let (expanded, heights) = (all("is_page_expanded"), all("page_height"));
    Some(file.views.iter().enumerate().map(|(k, v)| (v.clone(), expanded.get(k).is_none_or(|e| *e != "false"), heights.get(k).and_then(|h| h.parse().ok()).unwrap_or(-1.0))).collect())
}

/// The character's rollup area in the original's files ([`rollup_pages`]); none when the original has not played the
/// character on this machine.
pub fn rollup_file(character: u32) -> Option<Vec<(String, bool, f32)>> {
    let (_, dir) = prefs_dirs(character)?;
    std::fs::read_dir(dir.join("DockAreas")).ok()?.flatten().filter_map(|e| std::fs::read_to_string(e.path()).ok()).find_map(|t| rollup_pages(&t))
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
            if let Some(file) = dock_file(&text)
                && let Some(frame) = file.frame
            {
                for id in &file.views {
                    if let Some(h) = here(id) {
                        out.frames.insert(h, frame);
                    }
                }
                // (and the area's own, for the tab group made from it here: `docks.rs` keeps the original's names)
                if !file.name.is_empty() {
                    out.frames.insert(file.name.clone(), frame);
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
    // (the shortcut bars' frames, one file each)
    for n in 0..10 {
        if let Ok(text) = std::fs::read_to_string(dir.join("Containers").join(format!("ShortcutBar_{n}.xml")))
            && let Some(frame) = window_frame(&text)
        {
            out.frames.insert(format!("ShortcutBar_{n}"), frame);
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
        let one = dock_file(area).unwrap();
        assert_eq!((one.views, one.frame), (vec!["inventory_window".to_string()], Some([2709.0, 1233.0, 3290.0, 1408.0])));
        assert_eq!(rect("Rect(1,2,0,5)"), None);
        // (two views in one area: an array of identities, in the tabs' order)
        let two = r#"<Archive code="0">
    <Archive code="0" name="dock_config">
        <Array name="dock_node_configs">
            <Archive code="0" />
            <Archive code="0" />
        </Array>
        <Array name="docked_view_identities">
            <String value='&quot;inventory_window&quot;' />
            <String value='&quot;mission_window&quot;' />
        </Array>
        <Bool name="WindowPinButtonState" value="false" />
        <Rect name="WindowFrame" value="Rect(30.000000,98.000000,611.000000,273.000000)" />
        <Int32 name="selected_tab" value="1" />
    </Archive>
    <String name="dock_type" value='&quot;DockTabbedWindow&quot;' />
    <String name="dock_name" value='&quot;DockArea0&quot;' />
</Archive>"#;
        assert_eq!(dock_file(two), Some(DockFile { name: "DockArea0".into(), kind: "DockTabbedWindow".into(), views: vec!["inventory_window".into(), "mission_window".into()], selected: 1, frame: Some([30.0, 98.0, 611.0, 273.0]) }));
        let one = dock_file(area).unwrap();
        assert_eq!((one.selected, one.kind.as_str()), (0, "DockTabbedWindow"));
        // (a shortcut bar's file: its frame at the top, `bar_orientation` among the rest)
        let bar = "<Root>
    <Rect name=\"WindowFrame\" value=\"Rect(20.000000,20.000000,422.000000,57.000000)\" />
    <Int32 name=\"bar_orientation\" value=\"0\" />
</Root>";
        assert_eq!(window_frame(bar), Some([20.0, 20.0, 422.0, 57.0]));
    }

    /// A character's own file is found under any user's folder of any install, the one written last where two have
    /// it; a character the original never played here has none (no other character's stands in).
    #[test]
    fn a_characters_own_file_is_found_by_its_number() {
        let base = std::env::temp_dir().join(format!("mao-character-prefs-{}", std::process::id()));
        let (first, second) = (base.join("one/Prefs"), base.join("two/Prefs"));
        let files = [first.join("derived/Char5/Prefs.xml"), second.join("Someuser/Char5/Prefs.xml"), second.join("derived/Char6/Prefs.xml")];
        for (k, file) in files.iter().enumerate() {
            std::fs::create_dir_all(file.parent().unwrap()).unwrap();
            std::fs::write(file, "<Root />").unwrap();
            let written = std::time::SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(1_700_000_000 + 1000 * [2, 1, 3][k]);
            std::fs::File::options().write(true).open(file).unwrap().set_modified(written).unwrap();
        }
        let roots = [first.clone(), second.clone()];
        assert_eq!(character_prefs_in(&roots, 5), Some(files[0].clone()));
        assert_eq!(character_prefs_in(&roots[1..], 5), Some(files[1].clone()));
        assert_eq!(character_prefs_in(&roots, 6), Some(files[2].clone()));
        assert_eq!(character_prefs_in(&roots, 7), None);
        let _ = std::fs::remove_dir_all(&base);
    }

    /// A character's windows come from its own folder only; one the original never played has none, though another
    /// character's folder is there.
    #[test]
    fn a_characters_layout_is_its_own_or_none() {
        let base = std::env::temp_dir().join(format!("mao-prefs-dirs-{}", std::process::id()));
        let root = base.join("Prefs");
        std::fs::create_dir_all(root.join("derived/Char5")).unwrap();
        std::fs::write(root.join("derived/Char5/Prefs.xml"), "<Root />").unwrap();
        let roots = [root.clone()];
        assert_eq!(prefs_dirs_in(&roots, 5), Some((root.clone(), root.join("derived/Char5"))));
        assert_eq!(prefs_dirs_in(&roots, 6), None);
        let _ = std::fs::remove_dir_all(&base);
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

    /// The rollup area's file: its pages' views with their rolled state and height, in order; a tabbed area's is none.
    #[test]
    fn the_rollup_areas_pages_are_read() {
        let text = r#"<Archive code="0">
    <Archive code="0" name="dock_config">
        <Array name="dock_node_configs">
            <Archive code="0">
                <Bool name="is_page_expanded" value="true" />
                <Float name="page_height" value="103.000000" />
            </Archive>
            <Archive code="0">
                <Bool name="is_page_expanded" value="false" />
                <Float name="page_height" value="205.000000" />
            </Archive>
        </Array>
        <Array name="docked_view_identities">
            <String value='&quot;ncu_window&quot;' />
            <String value='&quot;nano_window&quot;' />
        </Array>
        <Float name="scroll_offset" value="0.000000" />
    </Archive>
    <String name="dock_type" value='&quot;RollupController&quot;' />
    <String name="dock_name" value='&quot;RollupArea&quot;' />
</Archive>"#;
        assert_eq!(rollup_pages(text), Some(vec![("ncu_window".to_string(), true, 103.0), ("nano_window".to_string(), false, 205.0)]));
        assert_eq!(rollup_pages(&text.replace("RollupController", "DockTabbedWindow")), None);
    }
}
